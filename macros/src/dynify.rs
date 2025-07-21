use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote, ToTokens};
use syn::{parse_quote, Error, Ident, Lifetime, Result, ReturnType, Token, Type};

use crate::utils::*;

pub fn expand(
    _attr: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> Result<TokenStream> {
    let input = TokenStream::from(input);
    let mut dyn_trait = syn::parse2::<syn::ItemTrait>(input.clone())?;
    let mut trait_impl_items = TokenStream::new();

    // TODO: support name customization
    // TODO: support non-trait items
    let dyn_trait_name = format_ident!("Dyn{}", dyn_trait.ident);
    let trait_name = std::mem::replace(&mut dyn_trait.ident, dyn_trait_name);
    let impl_target = format_ident!("{}Implementor", trait_name);

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
                let transformed = transform_fn(&dyn_trait.generics, sig, false)?;
                // TODO: support `#[dynify(skip)]`
                let attrs_outer = attrs.outer();
                let attrs_inner = attrs.inner();
                let impl_body = quote_transformed_body(transformed, &impl_target, sig);
                quote!(#(#attrs_outer)* #sig { #(#attrs_inner)* #impl_body })
            },
            _ => continue,
        };
        trait_impl_items.extend(impl_item);
    }

    let impl_generics = quote_impl_generics(&dyn_trait.generics);
    let dyn_trait_name = &dyn_trait.ident;
    Ok(quote!(
        #input
        #dyn_trait
        impl<#impl_generics #impl_target: #trait_name #ty_generics>
        #dyn_trait_name #ty_generics for #impl_target #where_clause {
            #trait_impl_items
        }
    ))
}

/// Generates implementation body for a transformed function.
fn quote_transformed_body(
    transformed: TransformResult,
    target: &Ident,
    sig: &syn::Signature,
) -> impl ToTokens {
    let ident = &sig.ident;
    let arg_idents = sig.inputs.iter().map(|arg| {
        quote_with(move |tokens| match arg {
            syn::FnArg::Receiver(r) => r.self_token.to_tokens(tokens),
            syn::FnArg::Typed(t) => t.pat.to_tokens(tokens),
        })
    });
    match transformed {
        TransformResult::Noop if sig.asyncness.is_some() => {
            quote!(#target::#ident(#(#arg_idents,)*).await)
        },
        TransformResult::Noop => {
            quote!(#target::#ident(#(#arg_idents,)*))
        },
        // TODO: expand macro calls
        TransformResult::Function => {
            quote!(::dynify::from_fn!(#target::#ident, #(#arg_idents,)*))
        },
        TransformResult::Method => {
            quote!(::dynify::from_fn!(#target::#ident, #(#arg_idents,)*))
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
                p.punct().map(|p| **p).unwrap_or_default().to_tokens(tokens);
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
    trait_generics: &syn::Generics,
    sig: &mut syn::Signature,
    force: bool,
) -> Result<TransformResult> {
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
        .map_while(|p| as_variant!(p, syn::GenericParam::Lifetime))
        .find(|l| l.lifetime.ident == "dynify")
        .map(|l| l.lifetime.clone());
    // Otherwise, insert a new one to the signature.
    let output_lifetime = output_lifetime.unwrap_or_else(|| {
        let lt = Lifetime::new("'dynify", Span::call_site());
        sig.generics.params.push(parse_quote!(#lt));
        lt
    });
    crate::lifetime::inject_output_lifetime(trait_generics, sig, &output_lifetime)?;

    // Infer the appropriate output type
    let input_types = {
        let this = sealed_recv
            .as_ref()
            .map::<Type, _>(|r| parse_quote!(::dynify::r#priv::#r));
        let args = sig
            .inputs
            .iter()
            .filter_map(|a| as_variant!(a, syn::FnArg::Typed).map(|t| Type::clone(&*t.ty)));
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
