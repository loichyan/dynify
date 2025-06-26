use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::Attribute;

pub(crate) struct QuoteWith<F>(F);
pub(crate) fn quote_with<F: Fn(&mut TokenStream)>(f: F) -> QuoteWith<F> {
    QuoteWith(f)
}
impl<F: Fn(&mut TokenStream)> ToTokens for QuoteWith<F> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        (self.0)(tokens)
    }
}

pub(crate) fn quote_outer(attrs: &[Attribute]) -> impl '_ + ToTokens {
    quote_with(move |tokens| {
        attrs
            .iter()
            .filter(|a| matches!(a.style, syn::AttrStyle::Outer))
            .for_each(|a| a.to_tokens(tokens))
    })
}

pub(crate) fn quote_inner(attrs: &[Attribute]) -> impl '_ + ToTokens {
    quote_with(move |tokens| {
        attrs
            .iter()
            .filter(|a| matches!(a.style, syn::AttrStyle::Inner(_)))
            .for_each(|a| a.to_tokens(tokens))
    })
}
