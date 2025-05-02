use proc_macro::TokenStream;
use quote::quote;
use syn::{token::{Brace, Paren}, *};

macro_rules! error {
    ($($tt:tt)*) => {
        return Err(syn::Error::new(proc_macro::Span::call_site().into(), format!($($tt)*)))
    };
}

pub fn from_row(input: DeriveInput) -> Result<TokenStream> {
    let DeriveInput { attrs: _, vis: _, ident, generics, data } = input;
    let Data::Struct(data) = data else {
        error!("only struct are currently supported")
    };

    let mut uses = quote! {};
    let mut head = quote! {};
    let mut matches = quote! {};
    let mut output = quote! {};

    match data.fields {
        Fields::Unnamed(FieldsUnnamed { unnamed, .. }) => {
            uses = quote! {use DecodeError::IndexOutOfBounds as Nope;};
            head = quote! { let mut iter = row.into_iter(); };
            let body = (0..unnamed.len())
                .map(|_|quote! { iter.try_next()?.decode()?, });
            Paren::default().surround(&mut output, |e|e.extend(body));
        },
        Fields::Named(FieldsNamed { named, .. }) => {
            uses = quote! {use DecodeError::ColumnNotFound as Nope;};
            head = named
                .iter()
                .map(|e|e.ident.as_ref().unwrap())
                .map(|e|(e.to_string(),e))
                .map(|(name,id)|quote! { let mut #id = Err(Nope(#name.into())); })
                .collect();
            let arms = named
                .iter()
                .map(|e|e.ident.as_ref().unwrap())
                .map(|e|(e.to_string(),e))
                .map(|(name,id)| quote! { #name => #id = Ok(col.decode()?), });
            matches = quote! {
                for column in row {
                    let col = column?;
                    match col.name() {
                        #(#arms)*
                        _ => {}
                    }
                }
            };
            let body = named
                .into_iter()
                .map(|e|e.ident.unwrap())
                .map(|id|quote! { #id: #id?, });
            Brace::default().surround(&mut output, |e|e.extend(body));
        }
        Fields::Unit => {}
    };

    let (g1, g2, g3) = generics.split_for_impl();

    Ok(quote! {
        impl #g1 ::postro::FromRow for #ident #g2 #g3 {
            fn from_row(row: ::postro::Row) -> Result<Self, postro::DecodeError> {
                #uses
                #head
                #matches
                Ok(Self #output)
            }
        }
    }.into())
}

