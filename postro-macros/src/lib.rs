use proc_macro::TokenStream;
use syn::DeriveInput;

mod from_row;

/// Foo
#[proc_macro_derive(FromRow)]
pub fn from_row(input: TokenStream) -> TokenStream {
    match from_row::from_row(syn::parse_macro_input!(input as DeriveInput)) {
        Ok(ok) => ok,
        Err(err) => err.into_compile_error().into(),
    }
}

