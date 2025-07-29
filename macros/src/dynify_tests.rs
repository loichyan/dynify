use proc_macro2::TokenStream;
use quote::quote;
use rstest::rstest;

use super::*;

define_macro_tests!(
    // === Items in Traits === //
    #[case::trait_empty(
        quote!(),
        quote!(trait Trait {}),
    )]
    #[case::trait_const_item(
        quote!(),
        quote!(trait Trait { const KST: usize; }),
    )]
    #[case::trait_type_item(
        quote!(),
        quote!(trait Trait { type Type: 'static; }),
    )]
    #[case::trait_async_method(
        quote!(),
        quote!(trait Trait { async fn test(&self, arg: &str); }),
    )]
    #[case::trait_impl_method(
        quote!(),
        quote!(trait Trait { fn test(&self, arg: &str) -> impl Any; }),
    )]
    #[case::trait_async_fn(
        quote!(),
        quote!(trait Trait { async fn test(this: &Self, arg: &str); }),
    )]
    #[case::trait_impl_fn(
        quote!(),
        quote!(trait Trait { fn test(this: &Self, arg: &str) -> impl Any; }),
    )]
    #[case::trait_regular_fn(
        quote!(),
        quote!(trait Trait { fn test(arg: &Self, arg: &str); }),
    )]
    #[case::trait_multi_items(
        quote!(),
        quote!(trait Trait {
            const KST1: usize;
            const KST2: bool;
            type Type1: 'static;
            type Type2: Future<Output = ()>;
            async fn method1(&self) -> Vec<u8>;
            fn method2(&self);
            async fn fun1(this: &Self) -> String;
            fn fun2(this: &Self) -> impl Future<Output = String>;
        }),
    )]
    // === Traits with Generics === //
    #[case::trait_with_generics(
        quote!(),
        quote!(trait Trait<'life1, 'life2, Arg1, Arg2> {
            const KST: usize;
            type Type: 'static;
            async fn method(&self);
            async fn fun(this: &Self);
        }),
    )]
    #[case::trait_with_where_clause(
        quote!(),
        quote!(trait Trait<'life1, 'life2>
        where
            'life2: 'life1,
            Self: 'static + Send,
        {
            const KST: usize;
            type Type: 'static;
            async fn method(&self);
            async fn fun(this: &Self);
        }),
    )]
    fn ui(#[case] test_name: &str, #[case] attr: TokenStream, #[case] input: TokenStream) {
        let output = expand(attr, input).unwrap();
        let output = prettyplease::unparse(&syn::parse_quote!(#output));
        validate_macro_output(&output, &format!("src/dynify_tests/{}.rs", test_name));
    }
);
