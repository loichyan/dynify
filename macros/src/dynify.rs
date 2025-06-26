use proc_macro2::TokenStream;
use quote::{format_ident, quote, quote_spanned, ToTokens};
use syn::spanned::Spanned;
use syn::{
    parse_quote_spanned, Error, FnArg, GenericArgument, Ident, ItemTrait, Path, PathArguments,
    Receiver, Result, ReturnType, Signature, Token, TraitItem, Type,
};

use crate::utils::*;

macro_rules! as_variant {
    ($variant:path, $val:expr) => {
        match $val {
            $variant(val) => Some(val),
            _ => None,
        }
    };
}

pub fn expand(
    _attr: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> Result<TokenStream> {
    let input = TokenStream::from(input);
    let mut trait_def = syn::parse2::<ItemTrait>(input.clone())?;
    let mut trait_impl_items = TokenStream::new();

    let trait_name = {
        let dyn_name = format_ident!("Dyn{}", trait_def.ident);
        std::mem::replace(&mut trait_def.ident, dyn_name)
    };
    let impl_target = format_ident!("{}Implementor", trait_name);

    for item in trait_def.items.iter_mut() {
        let TraitItem::Fn(func) = item else {
            return Err(Error::new_spanned(
                item,
                "non-function item is not supported yet",
            ));
        };

        let transformed = transform_fn(&mut func.sig, false)?;
        let fn_ident = &func.sig.ident;
        let span = fn_ident.span();

        let args = func.sig.inputs.iter().map(|arg| {
            quote_with(move |tokens| match arg {
                FnArg::Receiver(r) => r.self_token.to_tokens(tokens),
                FnArg::Typed(t) => t.pat.to_tokens(tokens),
            })
        });
        let impl_body = match transformed {
            TransformResult::Noop => quote_spanned!(span =>
                #impl_target::#fn_ident(#(#args,)*)
            ),
            // TODO: expand macro calls
            TransformResult::Function => quote_spanned!(span =>
                ::dynify::from_fn!(#impl_target::#fn_ident, #(#args,)*)
            ),
            TransformResult::Method => quote_spanned!(span =>
                ::dynify::from_fn!(#impl_target::#fn_ident, #(#args,)*)
            ),
        };

        let sig = &func.sig;
        let attrs_outer = quote_outer(&func.attrs);
        let attrs_inner = quote_inner(&func.attrs);
        trait_impl_items.extend(quote!(#attrs_outer #sig { #attrs_inner #impl_body }));
    }

    // TODO: generate dynified as a variant
    let dyn_trait_name = &trait_def.ident;
    Ok(quote!(
        #input
        #trait_def
        impl<#impl_target: #trait_name> #dyn_trait_name for #impl_target {
            #trait_impl_items
        }
    ))
}

enum TransformResult {
    Noop,
    Function,
    Method,
}

/// Transforms the supplied function into a dynified one, returning `true` only
/// if the transformation is successful.
fn transform_fn(sig: &mut Signature, force: bool) -> Result<TransformResult> {
    let span = sig.ident.span();

    if sig.asyncness.is_none() {
        // TODO: support `fn() -> impl Trait`
        if matches!(&sig.output, ReturnType::Type(_, t) if matches!(**t, Type::ImplTrait(_) )) {
            return Err(Error::new(span, "`impl Trait` is not supported yet"));
        }
        return Ok(TransformResult::Noop);
    }

    let result;
    let sealed_recv;
    match sig.receiver().map(infer_receiver) {
        Some(Some(r)) => {
            result = TransformResult::Method;
            sealed_recv = Some(r);
        },
        Some(None) if !force => {
            return Err(Error::new(
                span,
                "unsupported receiver type, use `#[dynify(force)]` or `#[dynify(skip)]` to suppress this error",
            ));
        },
        Some(None) | None => {
            result = TransformResult::Function;
            sealed_recv = None;
        },
    }

    let input_types = {
        let this =
            sealed_recv.map::<Type, _>(|r| parse_quote_spanned!(r.span() => ::dynify::r#priv::#r));
        let args = sig
            .inputs
            .iter()
            .filter_map(|a| as_variant!(syn::FnArg::Typed, a).map(|t| Type::clone(&*t.ty)));
        this.into_iter().chain(args)
    };

    let rarrow_token;
    let output_type: Type;
    match &sig.output {
        ReturnType::Default => {
            rarrow_token = Token![->](span);
            output_type = parse_quote_spanned!(span =>
                ::dynify::r#priv::Fn<(#(#input_types,)*), dyn '_ + ::core::future::Future<Output = ()>>
            );
        },
        ReturnType::Type(r, ty) => {
            rarrow_token = *r;
            output_type = parse_quote_spanned!(ty.span() =>
                ::dynify::r#priv::Fn<(#(#input_types,)*), dyn '_ + ::core::future::Future<Output = #ty>>
            );
        },
    };

    sig.output = ReturnType::Type(rarrow_token, Box::new(output_type));
    sig.asyncness = None;

    Ok(result)
}

fn infer_receiver(recv: &Receiver) -> Option<Ident> {
    let mut pinned = false;
    macro_rules! maybe_pinned {
        ($ty:ident) => {
            if pinned {
                concat!("Pin", stringify!($ty))
            } else {
                stringify!($ty)
            }
        };
    }

    let ty = as_variant!(Type::Path, &*recv.ty)
        .map(|ty| &ty.path)
        // Extract the inner type of `Pin<T>`
        .filter(|path| is_std(path, "pin", "Pin"))
        .map(|path| path.segments.last().unwrap())
        .and_then(|seg| extract_generic_arg(&seg.arguments))
        .inspect(|_| pinned = true)
        .unwrap_or(&recv.ty);

    let sealed = match ty {
        Type::Reference(r) => {
            if r.mutability.is_none() {
                maybe_pinned!(RefSelf)
            } else {
                maybe_pinned!(RefMutSelf)
            }
        },
        Type::Path(p) => {
            if is_std(&p.path, "boxed", "Box") {
                maybe_pinned!(BoxSelf)
            } else if is_std(&p.path, "rc", "Rc") {
                maybe_pinned!(RcSelf)
            } else if is_std(&p.path, "arc", "Arc") {
                maybe_pinned!(ArcSelf)
            } else {
                return None;
            }
        },
        _ => return None,
    };

    Some(Ident::new(sealed, ty.span()))
}

fn is_std(path: &Path, mod1: &str, ty: &str) -> bool {
    path.is_ident(ty)
        || path.segments.len() == 3
            && (path.segments[0].ident == "std" || path.segments[0].ident == "core")
            && path.segments[1].ident == mod1
            && path.segments[2].ident == ty
}

/// Extracts the first type generic argument.
fn extract_generic_arg(arg: &PathArguments) -> Option<&Type> {
    let arg = as_variant!(PathArguments::AngleBracketed, arg)?;
    let first = arg.args.first()?;
    as_variant!(GenericArgument::Type, first)
}
