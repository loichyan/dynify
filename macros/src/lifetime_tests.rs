use proc_macro2::TokenStream;
use quote::quote;
use rstest::rstest;

use super::*;
use crate::utils::*;

define_macro_tests!(
    // === Lifetimes in Arguments === //
    #[case::receiver(
        quote!(),
        quote!(fn test(&self)),
    )]
    #[case::typed_ref(
        quote!(),
        quote!(fn test(arg: &str)),
    )]
    #[case::receiver_with_placeholder_lifetime(
        quote!(),
        quote!(fn test(&'_ self)),
    )]
    #[case::typed_ref_with_placeholder_lifetime(
        quote!(),
        quote!(fn test(arg: &'_ str)),
    )]
    #[case::typed_path(
        quote!(),
        quote!(fn test(arg: Context<'_>)),
    )]
    #[case::typed_path_nested(
        quote!(),
        quote!(fn test(arg: Pin<&mut str>)),
    )]
    #[case::one_fn_with_multi_args(
        quote!(),
        quote!(fn test(&self, arg1: &str, arg2: Context<'_>)),
    )]
    #[case::one_arg_with_multi_lifetimes(
        quote!(),
        quote!(fn test(self: MySelf<'_, '_>, arg: (&str, &str))),
    )]
    // == Functions with Extra Generics == //
    #[case::fn_with_unused_lifetime(
        quote!(),
        quote!(fn test<'x>(&self)),
    )]
    #[case::fn_with_generic(
        quote!(),
        quote!(fn test<T>(arg: &str)),
    )]
    #[case::fn_with_const_generic(
        quote!(),
        quote!(fn test<const N: usize>(arg: &str)),
    )]
    #[case::fn_with_explicit_lifetimes(
        quote!(),
        quote!(fn test<'this1, 'arg20>(self: MySelf<'_, 'this1>, arg1: &str, arg2: (&'arg20 str, &str))),
    )]
    #[case::fn_with_multi_generics(
        quote!(),
        quote!(fn test<'this1, 'arg20, 'x, 'y, T, U, const N: usize, const M: bool>(
            self: MySelf<'_, 'this1>,
            arg1: &str,
            arg2: (&'arg20 str, &str),
        )),
    )]
    // == Edge Cases == //
    #[case::fn_pointer_as_arg(
        quote!(),
        quote!(fn test(&self, arg: fn(&str) -> &str)),
    )]
    // === Methods in A Trait === //
    #[case::method(
        quote!(trait Test {}),
        quote!(fn test(&self, arg: &str)),
    )]
    #[case::method_with_trait_generic(
        quote!(trait Test<Arg> {}),
        quote!(fn test(&self, arg: &str)),
    )]
    #[case::method_with_trait_lifetime(
        quote!(trait Test<'Life> {}),
        quote!(fn test(&self, arg: &str)),
    )]
    #[case::method_with_multi_trait_generics(
        quote!(trait Test<'Life1, 'Life2, Arg1, Arg2> {}),
        quote!(fn test(&self, arg: &str)),
    )]
    // === Bare Functions in A Trait === //
    #[case::trait_fn(
        quote!(trait Test {}),
        quote!(fn test(this: &Self, arg: &str)),
    )]
    #[case::trait_fn_with_trati_generic(
        quote!(trait Test<Arg> {}),
        quote!(fn test(this: &Self, arg: &str)),
    )]
    #[case::trait_fn_with_trait_lifetime(
        quote!(trait Test<'Life> {}),
        quote!(fn test(this: &Self, arg: &str)),
    )]
    #[case::trait_fn_with_multi_trait_generics(
        quote!(trait Test<'Life1, 'Life2, Arg1, Arg2> {}),
        quote!(fn test(this: &Self, arg: &str)),
    )]
    fn ui(#[case] test_name: &str, #[case] context: TokenStream, #[case] input: TokenStream) {
        let trait_context = match context.is_empty() {
            false => Some(syn::parse2::<syn::ItemTrait>(context).unwrap()),
            true => None,
        };
        let trait_context = trait_context.as_ref().map(|t| TraitContext {
            generics: &t.generics,
        });

        let mut input: syn::Signature = syn::parse2(input).unwrap();
        let output_lifetime = Lifetime::new("'dynify", Span::call_site());
        inject_output_lifetime(trait_context.as_ref(), &mut input, &output_lifetime).unwrap();

        let input = prettyplease::unparse(&syn::parse_quote!(#input {}));
        validate_macro_output(&input, &format!("src/lifetime_tests/{}.rs", test_name));
    }
);
