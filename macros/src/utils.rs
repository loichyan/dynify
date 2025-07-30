use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::Attribute;

macro_rules! as_variant {
    ($val:expr, $($path:ident)::+ $(,)?) => {
        match $val {
            $($path)::*(val) => Some(val),
            _ => None,
        }
    };
    ($val:expr, $($path:ident)::+ ($($field:ident),* $(,)?) $(,)?) => {
        match $val {
            $($path)::*($($field),*) => Some(($($field),*)),
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
pub(crate) fn is_std(path: &syn::Path, mod1: &str, mod2: &str, ty: &str) -> bool {
    let segments = &path.segments;
    segments.len() == 1 && segments[0].ident == ty
        || segments.len() == 3
            && (segments[0].ident == "std" || segments[0].ident == mod1)
            && segments[1].ident == mod2
            && segments[2].ident == ty
}

/// Extracts the inner type generic argument.
pub(crate) fn extract_inner_type(path: &syn::Path) -> Option<&syn::Type> {
    let segment = path.segments.last().unwrap();
    let args = &as_variant!(&segment.arguments, syn::PathArguments::AngleBracketed)?.args;
    if args.len() != 1 {
        None
    } else {
        as_variant!(&args[0], syn::GenericArgument::Type)
    }
}

/// Splits attributes into `#[outer]` and `#![inner]`.
pub(crate) trait AttrsExt<'a> {
    fn outer(self) -> impl Iterator<Item = &'a Attribute>;
    fn inner(self) -> impl Iterator<Item = &'a Attribute>;
}
impl<'a> AttrsExt<'a> for &'a [Attribute] {
    fn outer(self) -> impl Iterator<Item = &'a Attribute> {
        self.iter()
            .filter(|attr| matches!(attr.style, syn::AttrStyle::Outer))
    }

    fn inner(self) -> impl Iterator<Item = &'a Attribute> {
        self.iter()
            .filter(|attr| matches!(attr.style, syn::AttrStyle::Inner(_)))
    }
}

pub(crate) trait PairExt {
    type Value;
    type Punct;

    fn punct_or_default(&self) -> Self::Punct;
}
impl<T, P> PairExt for syn::punctuated::Pair<&T, &P>
where
    P: syn::token::Token + Copy + Default,
{
    type Value = T;
    type Punct = P;

    fn punct_or_default(&self) -> Self::Punct {
        self.punct().map(|&&p| p).unwrap_or_default()
    }
}

#[cfg(test)]
macro_rules! define_macro_tests {
    ($(#[case::$name:ident($($args:expr),* $(,)?)])* fn $($fun:tt)*) => {
        #[rstest]
        $(#[case::$name(stringify!($name), $($args),*)])*
        fn $($fun)*
    };
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
pub(crate) fn validate_macro_output(output: &str, path: &str) {
    use std::io::Write;

    let path: &std::path::Path = path.as_ref();
    if std::env::var("TRYBUILD").map_or(false, |v| v == "overwrite") {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).unwrap();
        }
        let mut f = std::fs::File::create(path).unwrap();
        writeln!(&mut f, "/* This file is @generated for testing purpose */").unwrap();
        f.write_all(output.as_bytes()).unwrap();
    } else {
        assert!(
            path.exists(),
            "missing output for '{}', try to update it with TRYBUILD=overwrite",
            path.file_name().unwrap().to_str().unwrap(),
        );
        let expected = std::fs::read_to_string(path).unwrap();
        let first_line_end = expected.find('\n').unwrap();
        let expected = &expected[(first_line_end + 1)..];
        pretty_assertions::assert_str_eq!(output, expected);
    }
}
