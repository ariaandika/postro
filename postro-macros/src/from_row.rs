use proc_macro::TokenStream;
use quote::quote;
use syn::*;
use crate::error;

pub fn from_row(input: DeriveInput) -> Result<TokenStream> {
    let DeriveInput { attrs: _, vis: _, ident, mut generics, data } = input;
    let Data::Struct(data) = data else {
        error!("only struct are currently supported")
    };

    let body = match data.fields {
        Fields::Unnamed(FieldsUnnamed { unnamed, .. }) => {
            let iter = (0..unnamed.len())
                .map(|_|quote! { iter.try_next()?.decode()?, });

            quote! {
                use ::postro::DecodeError::IndexOutOfBounds as Nope;
                let mut iter = row.into_iter();
                Ok(Self(#(#iter)*))
            }
        },
        Fields::Named(FieldsNamed { named, .. }) => {
            let vars = named
                .iter()
                .map(|e|e.ident.as_ref().unwrap())
                .map(|e|(e.to_string(),e))
                .map(|(name,id)|quote! { let mut #id = Err(Nope(#name.into())); });
            let arms = named
                .iter()
                .map(|e|e.ident.as_ref().unwrap())
                .map(|e|(e.to_string(),e))
                .map(|(name,id)| quote! { #name => #id = Ok(col.decode()?), });
            let iter = named
                .iter()
                .map(|e|e.ident.as_ref().unwrap())
                .map(|id|quote! { #id: #id?, });

            quote! {
                use ::postro::DecodeError::ColumnNotFound as Nope;
                #(#vars)*
                for column in row {
                    let col = column?;
                    match col.name() {
                        #(#arms)*
                        _ => {}
                    }
                }
                Ok(Self {
                    #(#iter)*
                })
            }
        }
        Fields::Unit => quote! {
            Ok(Self)
        }
    };

    for ty in generics.type_params_mut() {
        ty.bounds.push(syn::parse_quote!(::postro::Decode));
    }

    let (g1, g2, g3) = generics.split_for_impl();

    Ok(quote! {
        impl #g1 ::postro::FromRow for #ident #g2 #g3 {
            fn from_row(row: ::postro::Row) -> Result<Self, ::postro::DecodeError> {
                #body
            }
        }
    }.into())
}

