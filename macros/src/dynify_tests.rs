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
        quote!(trait Trait { fn test(&self, arg: &str) -> impl std::any::Any; }),
    )]
    #[case::trait_async_fn(
        quote!(),
        quote!(trait Trait { async fn test(this: &Self, arg: &str); }),
    )]
    #[case::trait_impl_fn(
        quote!(),
        quote!(trait Trait { fn test(this: &Self, arg: &str) -> impl std::any::Any; }),
    )]
    #[case::trait_regular_fn(
        quote!(),
        quote!(trait Trait { fn test(this: &Self, arg: &str); }),
    )]
    #[case::trait_multi_items(
        quote!(),
        quote!(trait Trait {
            const KST1: usize;
            const KST2: bool;
            type Type1: 'static;
            type Type2: core::future::Future<Output = ()>;
            async fn method1(&self) -> Vec<u8>;
            fn method2(&self);
            async fn fun1(this: &Self) -> String;
            fn fun2(this: &Self) -> impl core::future::Future<Output = String>;
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
    // == Traits with Customizations == //
    #[case::trait_with_name(
        quote!(MyDynTrait),
        quote!(trait Trait { async fn test(&self); }),
    )]
    // == Functions == //
    #[case::fn_with_vis(
        quote!(),
        quote!(pub(crate) fn test() -> impl core::any::Any { todo!() }),
    )]
    #[case::fn_returning_impl(
        quote!(),
        quote!(fn test() -> impl core::any::Any { todo!() }),
    )]
    #[case::fn_returning_async(
        quote!(),
        quote!(async fn test(_arg1: &str) -> String { todo!() }),
    )]
    #[case::fn_renamed(
        quote!(my_dyn_test),
        quote!(async fn test(_arg1: &str) -> String { todo!() }),
    )]
    // == Remote items == //
    #[case::remote_trait(
        quote!(remote = "dynify::r#priv::TestRemoteTrait"),
        quote!(trait DynTestRemoteTrait { async fn test(&self, arg: &str) -> usize; }),
    )]
    #[case::remote_fn(
        quote!(remote = "dynify::r#priv::test_remote_fn"),
        quote!(async fn dyn_test_remote_fn(_arg1: &str) -> usize {}),
    )]
    fn ui(#[case] test_name: &str, #[case] attr: TokenStream, #[case] input: TokenStream) {
        let output = expand(attr, input).unwrap();
        // Append `fn main() {}` so that they can pass compile tests
        let output = prettyplease::unparse(&syn::parse_quote!(#output fn main() {}));
        validate_macro_output(&output, &format!("src/dynify_tests/{}.rs", test_name));
    }
);
