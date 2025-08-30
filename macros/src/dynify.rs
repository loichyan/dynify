use proc_macro2::TokenStream;
use quote::{format_ident, quote, ToTokens};
use syn::{parse_quote_spanned, FnArg, Ident, Lifetime, Result, ReturnType, Token, Type};

use crate::lifetime::TraitContext;
use crate::utils::*;

pub fn expand(attr: TokenStream, input: TokenStream) -> Result<TokenStream> {
    let rename = syn::parse2::<Option<Ident>>(attr)?;
    let input_item = syn::parse2::<syn::Item>(input.clone())?;
    let output = match input_item {
        syn::Item::Trait(t) => expand_trait(rename, t)?,
        syn::Item::Fn(f) => expand_fn(rename, f)?,
        item => {
            return Err(syn::Error::new_spanned(
                &item,
                "expected a `fn` or `trait` item",
            ))
        },
    };
    Ok(quote!(#input #output))
}

fn expand_trait(rename: Option<Ident>, mut dyn_trait: syn::ItemTrait) -> Result<TokenStream> {
    let dyn_trait_name = rename.unwrap_or_else(|| format_ident!("Dyn{}", dyn_trait.ident));
    let input_trait_name = std::mem::replace(&mut dyn_trait.ident, dyn_trait_name);
    let dyn_trait_name = &dyn_trait.ident;

    let impl_target = format_ident!("{}Implementor", input_trait_name);
    let mut trait_impl_items = TokenStream::new();

    let (_, ty_generics, where_clause) = dyn_trait.generics.split_for_impl();
    for item in dyn_trait.items.iter_mut() {
        let impl_item = match item {
            syn::TraitItem::Const(syn::TraitItemConst {
                attrs,
                const_token,
                ident,
                colon_token,
                ty,
                semi_token,
                ..
            }) => {
                let attrs = attrs.outer();
                quote!(#(#attrs)* #const_token #ident #colon_token #ty
                    = #impl_target::#ident #semi_token)
            },
            syn::TraitItem::Type(syn::TraitItemType {
                attrs,
                type_token,
                ident,
                generics,
                semi_token,
                ..
            }) => {
                let attrs = attrs.outer();
                let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
                quote!(#(#attrs)* #type_token #ident #impl_generics
                    = #impl_target::#ident #ty_generics #where_clause #semi_token)
            },
            syn::TraitItem::Fn(syn::TraitItemFn { attrs, sig, .. }) => {
                let context = TraitContext {
                    generics: &dyn_trait.generics,
                };
                let transformed = transform_fn(Some(&context), sig, false)?;
                // TODO: support `#[dynify(skip)]`
                // TODO: support nested `#[dynify]`
                let attrs_outer = attrs.outer();
                let attrs_inner = attrs.inner();
                let target = quote_with(|tokens| {
                    impl_target.to_tokens(tokens);
                    NewToken![::].to_tokens(tokens);
                    sig.ident.to_tokens(tokens);
                });
                let impl_body = quote_transformed_body(transformed, &target, sig);
                quote!(#(#attrs_outer)* #sig { #(#attrs_inner)* #impl_body })
            },
            _ => continue,
        };
        trait_impl_items.extend(impl_item);
    }

    let impl_generics = quote_impl_generics(&dyn_trait.generics);
    Ok(quote!(
        #[allow(async_fn_in_trait)]
        #[allow(clippy::type_complexity)]
        #dyn_trait

        #[allow(clippy::type_complexity)]
        impl<#impl_generics #impl_target: #input_trait_name #ty_generics>
        #dyn_trait_name #ty_generics for #impl_target
        #where_clause { #trait_impl_items }
    ))
}

fn expand_fn(rename: Option<Ident>, mut dyn_fn: syn::ItemFn) -> Result<TokenStream> {
    let syn::ItemFn {
        vis,
        sig,
        attrs,
        block: _,
    } = &mut dyn_fn;

    let dyn_fn_name = rename.unwrap_or_else(|| format_ident!("dyn_{}", sig.ident));
    let input_fn_name = std::mem::replace(&mut sig.ident, dyn_fn_name);

    let transformed = transform_fn(None, sig, true)?;
    let attrs_outer = attrs.outer();
    let attrs_inner = attrs.inner();
    let impl_body = quote_transformed_body(transformed, &input_fn_name, sig);
    Ok(quote!(#(#attrs_outer)* #vis #sig { #(#attrs_inner)* #impl_body }))
}

/// Generates implementation body for a transformed function.
fn quote_transformed_body(
    transformed: TransformResult,
    target: &dyn ToTokens,
    sig: &syn::Signature,
) -> impl ToTokens {
    let arg_idents = sig.inputs.pairs().map(|p| {
        quote_with(move |tokens| {
            match p.value() {
                FnArg::Receiver(r) => r.self_token.to_tokens(tokens),
                FnArg::Typed(t) => t.pat.to_tokens(tokens),
            }
            p.punct_or_default().to_tokens(tokens);
        })
    });

    match transformed {
        TransformResult::Noop if sig.asyncness.is_some() => {
            quote!(#target (#(#arg_idents)*).await)
        },
        TransformResult::Noop => {
            quote!(#target (#(#arg_idents)*))
        },
        TransformResult::Function | TransformResult::Method => {
            let recv = sig.receiver().map(|r| &r.self_token);
            quote!(::dynify::__from_fn!([#recv] #target, #(#arg_idents)*))
        },
    }
}

/// Prints generics for implementation without angle brackets.
fn quote_impl_generics(generics: &syn::Generics) -> impl '_ + ToTokens {
    quote_with(move |tokens| {
        let is_lifetime = |p: &syn::GenericParam| matches!(p, syn::GenericParam::Lifetime(_));
        generics
            .params
            .pairs()
            .filter(|p| is_lifetime(p.value()))
            .chain(generics.params.pairs().filter(|p| !is_lifetime(p.value())))
            .for_each(|p| {
                p.value().to_tokens(tokens);
                p.punct_or_default().to_tokens(tokens);
            });
    })
}

#[derive(Clone, Copy)]
enum TransformResult {
    Noop,
    Function,
    Method,
}

/// Transforms the supplied function into a dynified one, returning `true` only
/// if the transformation is successful.
fn transform_fn(
    context: Option<&TraitContext>,
    sig: &mut syn::Signature,
    force: bool,
) -> Result<TransformResult> {
    let fn_span = sig.ident.span();
    if sig.asyncness.is_none() && get_impl_type(&sig.output).is_none() {
        if force {
            return Err(syn::Error::new(
                fn_span,
                "input function must return an `impl` type",
            ));
        } else {
            return Ok(TransformResult::Noop);
        }
    }

    let sealed_recv = match sig.receiver() {
        Some(r) => crate::receiver::infer_receiver(r)
            .ok_or_else(|| syn::Error::new(r.self_token.span, "unsupported receiver type"))
            .map(Some)?,
        None if force => None,
        None => return Ok(TransformResult::Noop),
    };

    let output_lifetime = Lifetime::new("'dynify", fn_span);
    crate::lifetime::inject_output_lifetime(context, sig, &output_lifetime)?;

    // Infer the appropriate output type
    let input_types = quote_with(|tokens| {
        sealed_recv
            .as_ref()
            .map(|r| quote!(::dynify::r#priv::#r,))
            .to_tokens(tokens);
        sig.inputs
            .pairs()
            .skip(sealed_recv.is_some() as usize)
            .for_each(|p| {
                match p.value() {
                    FnArg::Receiver(r) => r.ty.to_tokens(tokens),
                    FnArg::Typed(t) => t.ty.to_tokens(tokens),
                }
                p.punct_or_default().to_tokens(tokens);
            });
    });
    let output_type = match &sig.output {
        ReturnType::Default => ReturnType::Type(
            NewToken![->],
            parse_quote_spanned!(fn_span => ::dynify::r#priv::Fn<
                (#input_types),
                dyn #output_lifetime + ::core::future::Future<Output = ()>
            >),
        ),
        ReturnType::Type(r, ty) if sig.asyncness.is_some() => ReturnType::Type(
            *r,
            parse_quote_spanned!(fn_span => ::dynify::r#priv::Fn<
                (#input_types),
                dyn #output_lifetime + ::core::future::Future<Output = #ty>
            >),
        ),
        ty @ ReturnType::Type(..) => {
            let (r, ty) = get_impl_type(ty).unwrap();
            let bounds = ty
                .bounds
                .pairs()
                .filter(|p| !matches!(p.value(), syn::TypeParamBound::Lifetime(_)));
            ReturnType::Type(
                r,
                parse_quote_spanned!(fn_span => ::dynify::r#priv::Fn<
                    (#input_types),
                    dyn #output_lifetime + #(#bounds)*
                >),
            )
        },
    };

    sig.output = output_type;
    sig.asyncness = None;

    Ok(sealed_recv
        .map(|_| TransformResult::Method)
        .unwrap_or(TransformResult::Function))
}

fn get_impl_type(ty: &ReturnType) -> Option<(Token![->], &syn::TypeImplTrait)> {
    as_variant!(ty, ReturnType::Type(r, t))
        .and_then(|(r, ty)| as_variant!(&**ty, Type::ImplTrait).map(|ty| (*r, ty)))
}

#[cfg(test)]
#[path = "dynify_tests.rs"]
mod tests;
