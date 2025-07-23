use proc_macro2::TokenStream;
use quote::quote;
use rstest::{fixture, rstest};
use syn::{ItemTrait, Signature};

use super::*;

macro_rules! assert_ast_eq {
    ($left:expr, $right:expr) => {{
        let left = $left;
        let right = $right;
        let left = prettyplease::unparse(&left);
        let right = prettyplease::unparse(&right);
        pretty_assertions::assert_eq!(left, right);
    }};
}

#[fixture]
fn output_lifetime() -> Lifetime {
    Lifetime::new("'dynify", Span::call_site())
}

// TODO: write expected output to files

#[rstest]
// === Lifetimes in Arguments === //
#[case::receiver(
    quote!(fn test(&self)),
    quote!(fn test<'this, 'dynify>(&'this self) where 'this: 'dynify),
)]
#[case::typed_ref(
    quote!(fn test(arg: &str)),
    quote!(fn test<'arg, 'dynify>(arg: &'arg str) where 'arg: 'dynify),
)]
#[case::receiver_with_underscore(
    quote!(fn test(&'_ self)),
    quote!(fn test<'this, 'dynify>(&'this self) where 'this: 'dynify),
)]
#[case::typed_ref_with_underscore(
    quote!(fn test(arg: &'_ str)),
    quote!(fn test<'arg, 'dynify>(arg: &'arg str) where 'arg: 'dynify),
)]
#[case::typed_path(
    quote!(fn test(arg: Context<'_>)),
    quote!(fn test<'arg, 'dynify>(arg: Context<'arg>) where 'arg: 'dynify),
)]
#[case::typed_path_nested(
    quote!(fn test(arg: Pin<&mut str>)),
    quote!(fn test<'arg, 'dynify>(arg: Pin<&'arg mut str>) where 'arg: 'dynify),
)]
#[case::multi_args(
    quote!(fn test(&self, arg1: &str, arg2: Context<'_>)),
    quote!(
        fn test<'this, 'arg1, 'arg2, 'dynify>(
            &'this self,
            arg1: &'arg1 str,
            arg2: Context<'arg2>
        ) where
            'this: 'dynify,
            'arg1: 'dynify,
            'arg2: 'dynify,
    ),
)]
#[case::one_arg_multi_lifetimes(
    quote!(fn test(self: MySelf<'_, '_>, arg: (&str, &str))),
    quote!(
        fn test<'this0, 'this1, 'arg0, 'arg1, 'dynify>(
            self: MySelf<'this0, 'this1>,
            arg: (&'arg0 str, &'arg1 str),
        )
        where
            'this0: 'dynify,
            'this1: 'dynify,
            'arg0 : 'dynify,
            'arg1 : 'dynify,
    ),
)]
// == Functions with Extra Generics == //
#[case::extra_unused_lifetimes(
    quote!(fn test<'x>(&self)),
    quote!(fn test<'x, 'this, 'dynify>(&'this self) where 'this: 'dynify),
)]
#[case::extra_generics(
    quote!(fn test<T>(arg: &str)),
    quote!(fn test<'arg, 'dynify, T>(arg: &'arg str) where 'arg: 'dynify, T: 'dynify),
)]
#[case::extra_const_generics(
    quote!(fn test<const N: usize, T>(arg: &str)),
    quote!(fn test<'arg, 'dynify, const N: usize, T>(arg: &'arg str) where 'arg: 'dynify, T: 'dynify),
)]
#[case::extra_explicit(
    quote!(fn test<'this1, 'arg20>(self: MySelf<'_, 'this1>, arg1: &str, arg2: (&'arg20 str, &str))),
    quote!(
        fn test<'this1, 'arg20, 'this0, 'arg1, 'arg21, 'dynify>(
            self: MySelf<'this0, 'this1>,
            arg1: &'arg1 str,
            arg2: (&'arg20 str, &'arg21 str),
        )
        where
            'arg20: 'dynify,
            'this1: 'dynify,
            'this0: 'dynify,
            'arg1 : 'dynify,
            'arg21: 'dynify,
    ),
)]
// == Edge Cases == //
#[case::fn_pointer_arg(
    quote!(fn test(&self, arg: fn(&str) -> &str)),
    quote!(fn test<'this, 'dynify>(&'this self, arg: fn(&str) -> &str) where 'this: 'dynify),
)]
fn injected_lifetimes(
    #[case] input: TokenStream,
    #[case] expected: TokenStream,
    output_lifetime: Lifetime,
) {
    let mut input: Signature = syn::parse2(input.clone()).unwrap();
    inject_output_lifetime(None, &mut input, &output_lifetime).unwrap();
    assert_ast_eq!(parse_quote!(#input {}), parse_quote!(#expected {}));
}

#[rstest]
// === Methods in A Trait === //
#[case::method(
    quote!(trait Test {}),
    quote!(fn test(&self, arg: &str)),
    quote!(
        fn test<'this, 'arg, 'dynify>(&'this self, arg: &'arg str)
        where
            'this: 'dynify,
            'arg: 'dynify,
            Self: 'dynify,
    ),
)]
#[case::method_with_typed(
    quote!(trait Test<Arg> {}),
    quote!(fn test(&self, arg: &str)),
    quote!(
        fn test<'this, 'arg, 'dynify>(&'this self, arg: &'arg str)
        where
            'this: 'dynify,
             'arg: 'dynify,
              Arg: 'dynify,
             Self: 'dynify,
    ),
)]
#[case::method_with_lifetime(
    quote!(trait Test<'Life> {}),
    quote!(fn test(&self, arg: &str)),
    quote!(
        fn test<'this, 'arg, 'dynify>(&'this self, arg: &'arg str)
        where
            'this: 'dynify,
             'arg: 'dynify,
            'Life: 'dynify,
             Self: 'dynify,
    ),
)]
#[case::method_with_multi(
    quote!(trait Test<'Life1, 'Life2, Arg1, Arg2> {}),
    quote!(fn test(&self, arg: &str)),
    quote!(
        fn test<'this, 'arg, 'dynify>(&'this self, arg: &'arg str)
        where
             'this: 'dynify,
              'arg: 'dynify,
            'Life1: 'dynify,
            'Life2: 'dynify,
              Arg1: 'dynify,
              Arg2: 'dynify,
              Self: 'dynify,
    ),
)]
// === Bare Functions in A Trait === //
#[case::bare_fn(
    quote!(trait Test {}),
    quote!(fn test(this: &Self, arg: &str)),
    quote!(
        fn test<'this, 'arg, 'dynify>(this: &'this Self, arg: &'arg str)
        where
            'this: 'dynify,
            'arg: 'dynify,
    ),
)]
#[case::bare_fn_with_typed(
    quote!(trait Test<Arg> {}),
    quote!(fn test(this: &Self, arg: &str)),
    quote!(
        fn test<'this, 'arg, 'dynify>(this: &'this Self, arg: &'arg str)
        where
            'this: 'dynify,
             'arg: 'dynify,
              Arg: 'dynify,
    ),
)]
#[case::bare_fn_with_lifetime(
    quote!(trait Test<'Life> {}),
    quote!(fn test(this: &Self, arg: &str)),
    quote!(
        fn test<'this, 'arg, 'dynify>(this: &'this Self, arg: &'arg str)
        where
            'this: 'dynify,
             'arg: 'dynify,
            'Life: 'dynify,
    ),
)]
#[case::bare_fn_with_multi(
    quote!(trait Test<'Life1, 'Life2, Arg1, Arg2> {}),
    quote!(fn test(this: &Self, arg: &str)),
    quote!(
        fn test<'this, 'arg, 'dynify>(this: &'this Self, arg: &'arg str)
        where
             'this: 'dynify,
              'arg: 'dynify,
            'Life1: 'dynify,
            'Life2: 'dynify,
              Arg1: 'dynify,
              Arg2: 'dynify,
    ),
)]
fn injected_lifetimes_in_trait(
    #[case] context: TokenStream,
    #[case] input: TokenStream,
    #[case] expected: TokenStream,
    output_lifetime: Lifetime,
) {
    let mut input: Signature = syn::parse2(input.clone()).unwrap();
    let trait_context: ItemTrait = syn::parse2(context).unwrap();
    let trait_context = TraitContext {
        name: &trait_context.ident,
        generics: &trait_context.generics,
    };
    inject_output_lifetime(Some(&trait_context), &mut input, &output_lifetime).unwrap();
    assert_ast_eq!(parse_quote!(#input {}), parse_quote!(#expected {}));
}
