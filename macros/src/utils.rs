use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::Attribute;

macro_rules! as_variant {
    ($val:expr, $variant:path $(,)?) => {
        match $val {
            $variant(val) => Some(val),
            _ => None,
        }
    };
}

pub(crate) fn quote_with<F: Fn(&mut TokenStream)>(f: F) -> QuoteWith<F> {
    QuoteWith(f)
}
pub(crate) struct QuoteWith<F>(F);
impl<F: Fn(&mut TokenStream)> ToTokens for QuoteWith<F> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        (self.0)(tokens)
    }
}

/// Determines whether the supplied path matches an item in `std`.
pub(crate) fn is_std(path: &syn::Path, mod1: &str, ty: &str) -> bool {
    path.is_ident(ty)
        || path.segments.len() == 3
            && (path.segments[0].ident == "std" || path.segments[0].ident == "core")
            && path.segments[1].ident == mod1
            && path.segments[2].ident == ty
}

/// Extracts the first type generic argument.
pub(crate) fn extract_inner_type(arg: &syn::PathArguments) -> Option<&syn::Type> {
    let arg = as_variant!(arg, syn::PathArguments::AngleBracketed)?;
    let first = arg.args.first()?;
    as_variant!(first, syn::GenericArgument::Type)
}

/// Splits attributes into `#[outer]` and `#![inner]`.
pub(crate) trait FilterAttrs<'a> {
    type Filtered: Iterator<Item = &'a Attribute>;

    fn outer(self) -> Self::Filtered;
    fn inner(self) -> Self::Filtered;
}
impl<'a> FilterAttrs<'a> for &'a [Attribute] {
    type Filtered = std::iter::Filter<std::slice::Iter<'a, Attribute>, fn(&&Attribute) -> bool>;

    fn outer(self) -> Self::Filtered {
        self.iter()
            .filter(|attr| matches!(attr.style, syn::AttrStyle::Outer))
    }

    fn inner(self) -> Self::Filtered {
        self.iter()
            .filter(|attr| matches!(attr.style, syn::AttrStyle::Inner(_)))
    }
}
