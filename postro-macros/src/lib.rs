use proc_macro::TokenStream;
use syn::DeriveInput;

mod from_row;
mod table;
mod decode;

/// Automatically derive [`FromRow`].
#[proc_macro_derive(FromRow)]
pub fn from_row(input: TokenStream) -> TokenStream {
    match from_row::from_row(syn::parse_macro_input!(input as DeriveInput)) {
        Ok(ok) => ok,
        Err(err) => err.into_compile_error().into(),
    }
}

/// Automatically derive [`Table`].
#[proc_macro_derive(Table,attributes(sql))]
pub fn table(input: TokenStream) -> TokenStream {
    match table::table(syn::parse_macro_input!(input as DeriveInput)) {
        Ok(ok) => ok,
        Err(err) => err.into_compile_error().into(),
    }
}

/// Automatically derive [`Decode`].
#[proc_macro_derive(Decode)]
pub fn decode(input: TokenStream) -> TokenStream {
    match decode::decode(syn::parse_macro_input!(input as DeriveInput)) {
        Ok(ok) => ok,
        Err(err) => err.into_compile_error().into(),
    }
}

macro_rules! error {
    ($($tt:tt)*) => {
        return Err(syn::Error::new(proc_macro::Span::call_site().into(), format!($($tt)*)))
    };
}

pub(crate) use error;

