use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote, ToTokens};
use syn::{parse_quote, Error, Lifetime, Result, ReturnType, Token, Type};

use crate::utils::*;

pub fn expand(
    _attr: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> Result<TokenStream> {
    let input = TokenStream::from(input);
    let mut trait_def = syn::parse2::<syn::ItemTrait>(input.clone())?;
    let mut trait_impl_items = TokenStream::new();

    let trait_name = {
        let dyn_name = format_ident!("Dyn{}", trait_def.ident);
        std::mem::replace(&mut trait_def.ident, dyn_name)
    };
    let impl_target = format_ident!("{}Implementor", trait_name);

    for item in trait_def.items.iter_mut() {
        let syn::TraitItem::Fn(func) = item else {
            return Err(Error::new_spanned(
                item,
                "non-function item is not supported yet",
            ));
        };

        let transformed = transform_fn(&mut func.sig, false)?;
        let fn_ident = &func.sig.ident;

        let args = func.sig.inputs.iter().map(|arg| {
            quote_with(move |tokens| match arg {
                syn::FnArg::Receiver(r) => r.self_token.to_tokens(tokens),
                syn::FnArg::Typed(t) => t.pat.to_tokens(tokens),
            })
        });
        let impl_body = match transformed {
            TransformResult::Noop if func.sig.asyncness.is_some() => {
                quote!(#impl_target::#fn_ident(#(#args,)*).await)
            },
            TransformResult::Noop => {
                quote!(#impl_target::#fn_ident(#(#args,)*))
            },
            // TODO: expand macro calls
            TransformResult::Function => {
                quote!(::dynify::from_fn!(#impl_target::#fn_ident, #(#args,)*))
            },
            TransformResult::Method => {
                quote!(::dynify::from_fn!(#impl_target::#fn_ident, #(#args,)*))
            },
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
        // TODO: handle trait generics
        impl<#impl_target: #trait_name> #dyn_trait_name for #impl_target {
            #trait_impl_items
        }
    ))
}

#[derive(Clone, Copy)]
enum TransformResult {
    Noop,
    Function,
    Method,
}

/// Transforms the supplied function into a dynified one, returning `true` only
/// if the transformation is successful.
fn transform_fn(sig: &mut syn::Signature, force: bool) -> Result<TransformResult> {
    let span = sig.ident.span();

    if sig.asyncness.is_none() {
        // TODO: support `fn() -> impl Trait`
        if matches!(&sig.output, ReturnType::Type(_, t) if matches!(**t, Type::ImplTrait(_) )) {
            return Err(Error::new(span, "`impl Trait` is not supported yet"));
        }
        return Ok(TransformResult::Noop);
    }

    let sealed_recv = match sig.receiver().map(crate::receiver::infer_receiver) {
        Some(Some(r)) => Some(r),
        Some(None) => return Err(Error::new(span, "cannot determine receiver type")),
        None if force => None,
        None => return Ok(TransformResult::Noop),
    };

    // If `'dynify` is already specified, use it directly.
    let output_lifetime = sig
        .generics
        .params
        .iter()
        .map_while(|p| as_variant!(syn::GenericParam::Lifetime, p))
        .find(|l| l.lifetime.ident == "dynify")
        .map(|l| l.lifetime.clone());
    // Otherwise, insert a new one to the signature.
    let output_lifetime = output_lifetime.unwrap_or_else(|| {
        let lt = Lifetime::new("'dynify", Span::call_site());
        sig.generics.params.push(parse_quote!(#lt));
        lt
    });
    crate::lifetime::inject_output_lifetime(sig, &output_lifetime)?;

    // Infer the appropriate output type
    let input_types = {
        let this = sealed_recv
            .as_ref()
            .map::<Type, _>(|r| parse_quote!(::dynify::r#priv::#r));
        let args = sig
            .inputs
            .iter()
            .filter_map(|a| as_variant!(syn::FnArg::Typed, a).map(|t| Type::clone(&*t.ty)));
        this.into_iter().chain(args)
    };
    let output_type = match &sig.output {
        ReturnType::Default => ReturnType::Type(
            <Token![->]>::default(),
            parse_quote!(
                ::dynify::r#priv::Fn<(#(#input_types,)*), dyn #output_lifetime + ::core::future::Future<Output = ()>>
            ),
        ),
        ReturnType::Type(r, ty) => ReturnType::Type(
            *r,
            parse_quote!(
                ::dynify::r#priv::Fn<(#(#input_types,)*), dyn #output_lifetime + ::core::future::Future<Output = #ty>>
            ),
        ),
    };

    sig.output = output_type;
    sig.asyncness = None;

    Ok(sealed_recv
        .map(|_| TransformResult::Method)
        .unwrap_or(TransformResult::Function))
}
