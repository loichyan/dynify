use proc_macro::TokenStream;

mod dynify;
mod utils;

#[proc_macro_attribute]
pub fn dynify(attr: TokenStream, input: TokenStream) -> TokenStream {
    dynify::expand(attr, input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}
