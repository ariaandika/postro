use proc_macro::TokenStream;
use quote::quote;
use syn::*;
use crate::error;

pub fn table(input: DeriveInput) -> Result<TokenStream> {
    let DeriveInput { attrs, vis: _, ident, generics, data } = input;
    let Data::Struct(data) = data else {
        error!("only struct are supported")
    };

    let table = attrs
        .iter()
        .find(|e| e.path().is_ident("sql"))
        .map(|e| Ok::<_, Error>(e.parse_args::<LitStr>()?.value()))
        .unwrap_or_else(|| Ok(to_snake_case(&ident.to_string())))?;

    let insert = match data.fields {
        Fields::Named(FieldsNamed { named, .. }) => {
            let opts = named
                .iter()
                .map(AttributeType::from_field)
                .collect::<Result<Vec<_>>>()?;

            let fields = named
                .iter()
                .zip(opts.iter())
                .filter(|(_,attr)|!matches!(attr,AttributeType::Id))
                .map(|(id,_)|id.ident.as_ref().map(<_>::to_string).unwrap_or_default())
                .collect::<Vec<_>>()
                .join(",");

            let params = opts
                .into_iter()
                .filter(|attr|!matches!(attr,AttributeType::Id))
                .scan(1, |state,attr|{
                    match attr {
                        AttributeType::Id => unreachable!(),
                        AttributeType::None => {
                            let id = format!("${state}");
                            *state += 1;
                            Some(id)
                        }
                        AttributeType::Sql(sql) => {
                            Some(sql)
                        }
                    }
                })
                .collect::<Vec<_>>()
                .join(",");

            format!("INSERT INTO {table}({fields}) VALUES({params})")
        },
        _ => error!("only named struct are supported"),
    };

    let (g1, g2, g3) = generics.split_for_impl();

    Ok(quote! {
        impl #g1 ::postro::Table for #ident #g2 #g3 {
            const TABLE: &str = #table;

            const INSERT: &str = #insert;
        }
    }.into())
}

pub fn to_snake_case(string: &str) -> String {
    if string.is_empty() {
        return String::new();
    }

    let mut output = String::with_capacity(string.len());

    let mut iter = string.chars();
    let Some(lead) = iter.next() else {
        unreachable!()
    };

    output.extend(lead.to_lowercase());

    for it in iter {
        if it.is_uppercase() {
            output.push('_');
            output.extend(it.to_lowercase());
        } else {
            output.push(it);
        }
    }

    output
}

#[derive(Debug)]
enum AttributeType {
    /// no attribute
    None,
    /// `#[sql(id)]`
    Id,
    /// `#[sql("now()")]`
    Sql(String),
}

impl AttributeType {
    fn from_field(field: &Field) -> Result<Self> {
        field
            .attrs
            .iter()
            .find(|attr| attr.path().is_ident("sql"))
            .map(|attr| {
                attr.parse_args_with(|e: parse::ParseStream| {
                    let look = e.lookahead1();
                    if look.peek(Ident) {
                        if matches!(e.parse::<Ident>()?.to_string().as_str(), "id" | "skip") {
                            Ok(Self::Id)
                        } else {
                            error!("possible value are: `id`, `skip` or `\"sql statement\"`")
                        }
                    } else if look.peek(LitStr) {
                        Ok(Self::Sql(e.parse::<LitStr>()?.value()))
                    } else {
                        Err(look.error())
                    }
                })
            })
            .unwrap_or(Ok(Self::None))
    }
}

