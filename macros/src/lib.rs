//! Procedural macros for [dynify](https://crates.io/crates/dynify).
//!
//! You should not use this crate directly. All macros and documentations are
//! available in dynify with its `macros` feature enabled.

#![cfg_attr(coverage_nightly, feature(coverage_attribute))]

use proc_macro::TokenStream;

#[macro_use]
mod utils;
mod dynify;
mod lifetime;
mod receiver;

#[proc_macro_attribute]
pub fn dynify(attr: TokenStream, input: TokenStream) -> TokenStream {
    dynify::expand(attr.into(), input.into())
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}
