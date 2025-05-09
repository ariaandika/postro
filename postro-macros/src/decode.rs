use proc_macro::TokenStream;
use quote::quote;
use syn::*;
use crate::error;

pub fn decode(input: DeriveInput) -> Result<TokenStream> {
    let DeriveInput { attrs: _, vis: _, ident, mut generics, data } = input;

    let q1 = match data {
        Data::Struct(st) => match &st.fields {
            Fields::Unnamed(FieldsUnnamed { unnamed, .. }) => {
                if unnamed.len() != 1 {
                    error!("only one field struct is supported")
                }

                quote! {
                    Ok(Self(col.decode()?))
                }
            }
            Fields::Named(FieldsNamed { named, .. }) => {
                if named.len() != 1 {
                    error!("only one field struct is supported")
                }

                let name = named.first().unwrap().ident.as_ref().unwrap();

                quote! {
                    Ok(Self {
                        #name: col.decode()?,
                    })
                }
            },
            Fields::Unit => quote! { Ok(Self) }
        },
        Data::Enum(_) => error!("union is not yet supported"),
        Data::Union(_) => error!("union is not supported"),
    };

    for ty in generics.type_params_mut() {
        ty.bounds.push(syn::parse_quote!(::postro::Decode));
    }

    let (g1, g2, g3) = generics.split_for_impl();

    Ok(quote! {
        #[automatically_derived]
        impl #g1 ::postro::Decode for #ident #g2 #g3 {
            fn decode(col: ::postro::row::Column) -> Result<Self, ::postro::DecodeError> {
                #q1
            }
        }
    }.into())
}

pub fn encode(input: DeriveInput) -> Result<TokenStream> {
    let DeriveInput { attrs: _, vis: _, ident, generics, data } = input;

    let mut gt = generics.clone();

    if gt.lifetimes().next().is_none() {
        gt.params.push(syn::parse_quote!('__encode));
    }

    let lt = gt.lifetimes().next().cloned().unwrap();

    let q1 = match data {
        Data::Struct(st) => match &st.fields {
            Fields::Unnamed(FieldsUnnamed { unnamed, .. }) => {
                if unnamed.len() != 1 {
                    error!("only one field struct is supported")
                }

                quote! {
                    self.0.encode()
                }
            }
            Fields::Named(FieldsNamed { named, .. }) => {
                if named.len() != 1 {
                    error!("only one field struct is supported")
                }

                let name = named.first().unwrap().ident.as_ref().unwrap();

                quote! { self.#name.encode() }
            },
            Fields::Unit => quote! { ::postro::encode::Encoded::null() }
        },
        Data::Enum(_) => error!("union is not yet supported"),
        Data::Union(_) => error!("union is not supported"),
    };

    let idents = gt.type_params().map(|e|e.ident.clone()).collect::<Vec<_>>();

    for ident in idents {
        gt.make_where_clause().predicates.push(syn::parse_quote!(& #lt #ident: ::postro::Encode<#lt>));
    }

    let (g1, _, g3) = gt.split_for_impl();
    let (_, g2, _) = generics.split_for_impl();

    Ok(quote! {
        #[automatically_derived]
        impl #g1 ::postro::Encode<#lt> for & #lt #ident #g2 #g3 {
            fn encode(self) -> ::postro::encode::Encoded<#lt> {
                #q1
            }
        }
    }.into())
}

