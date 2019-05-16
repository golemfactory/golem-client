extern crate proc_macro;

use failure::{bail, Fallible};
use heck::{CamelCase, ShoutySnakeCase, SnakeCase};
use proc_macro::TokenStream;
use quote::quote;
use regex::Regex;
use std::convert::{TryFrom, TryInto};
use std::str::FromStr;
use syn::{parse_macro_input, File, Lit, Type, Expr};

// Model
struct SettingSection {
    // group name
    name: syn::Ident,
    items: Vec<SettingDef>,
}

struct SettingDef {
    // Setting as type name (eg. NodeName for node_name setting).
    type_name: syn::Ident,
    // Setting name as const name (eg. NODE_NAME for node_name setting).
    kw_name: syn::Ident,
    // Setting native type.
    ty: syn::Type,
    // Description for user.
    desc: String,
    // Setting name as string.
    name: String,
    //
    conversion_type: ConversionType,
    validation: Option<Range<Expr>>,
}

enum ConversionType {
    String,
    // Natural numbers 0, 1, 2,...
    // usize
    Nat,
    // BigDecimal
    Decimal,
    Bool,
    Float,
}

enum Units {
    Gnt,
    Other(String),
}

#[derive(Debug)]
enum RangeEnd<T> {
    Inclusive(T),
    Exclusive(T),
}

impl<T> RangeEnd<T> {

    fn as_gt_str(&self) -> &str {
        match self {
            RangeEnd::Inclusive(_) => ">=",
            RangeEnd::Exclusive(_) => ">"
        }
    }

    fn as_lt_str(&self) -> &str {
        match self {
            RangeEnd::Inclusive(_) => "<=",
            RangeEnd::Exclusive(_) => "<"
        }
    }

    fn as_val(&self) -> &T {
        match self {
            RangeEnd::Inclusive(v) => v,
            RangeEnd::Exclusive(v) => v
        }
    }

}

#[derive(Debug)]
struct Range<T> {
    from: Option<RangeEnd<T>>,
    to: Option<RangeEnd<T>>,
}

impl FromStr for Range<Expr> {
    type Err = failure::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let re_both = Regex::new(r"([-0-9.]+)\s*(<|<=)\s*v\s*(<|<=)\s*([0-9.]+)").unwrap();
        let re_one = Regex::new(r"\s*v\s*(<|<=|>|>=)\s*([-0-9.]+)").unwrap();
        lazy_static::lazy_static! {
            static ref EXAMPLE: u8 = 42;
        }

        if let Some(m) = re_both.captures(s) {
            let left: Expr = syn::parse_str(m.get(1).unwrap().as_str())?;
            let from = Some(if m.get(2).unwrap().as_str() == "<=" {
                RangeEnd::Inclusive(left)
            } else {
                RangeEnd::Exclusive(left)
            });
            let right: Expr = syn::parse_str(m.get(4).unwrap().as_str())?;
            let to = Some(if m.get(3).unwrap().as_str() == "<=" {
                RangeEnd::Inclusive(right)
            } else {
                RangeEnd::Exclusive(right)
            });
            return Ok(Range { from, to });
        } else if let Some(m) = re_one.captures(s) {
            let end_val: Expr = syn::parse_str(m.get(2).unwrap().as_str())?;
            return Ok(match m.get(1).unwrap().as_str() {
                "<" => Range {
                    from: None,
                    to: Some(RangeEnd::Exclusive(end_val)),
                },
                "<=" => Range {
                    from: None,
                    to: Some(RangeEnd::Inclusive(end_val)),
                },
                ">" => Range {
                    from: Some(RangeEnd::Exclusive(end_val)),
                    to: None,
                },
                ">=" => Range {
                    from: Some(RangeEnd::Inclusive(end_val)),
                    to: None,
                },
                _ => unreachable!(),
            });
        }

        bail!("not implemented check: {}", s)
    }
}

impl TryFrom<&syn::Type> for ConversionType {
    type Error = failure::Error;

    fn try_from(value: &syn::Type) -> Result<Self, Self::Error> {
        match value {
            syn::Type::Path(p) => {
                if p.path.is_ident("usize") {
                    return Ok(ConversionType::Nat);
                }
                if p.path.is_ident("bool") {
                    return Ok(ConversionType::Bool);
                }
                if p.path.is_ident("String") {
                    return Ok(ConversionType::String);
                }
                if p.path.is_ident("BigDecimal") {
                    return Ok(ConversionType::Decimal);
                }
                if p.path.is_ident("f64") {
                    return Ok(ConversionType::Float);
                }
                Err(failure::err_msg(format!("Unsupported type: {:?}", p)))
            }
            v => Err(failure::err_msg(format!("Unsupported type: {:?}", v))),
        }
    }
}

fn parse_section(s: &syn::ItemStruct) -> Fallible<SettingSection> {
    let name = syn::Ident::new(&s.ident.to_string().to_snake_case(), s.ident.span());
    let items = s
        .fields
        .iter()
        .map(|f| parse_section_field(f))
        .collect::<Result<Vec<_>, failure::Error>>()?;

    Ok::<_, failure::Error>(SettingSection { name, items })
}

fn parse_section_field(f: &syn::Field) -> Fallible<SettingDef> {
    let ident = f.ident.clone().unwrap();
    let name = ident.to_string();
    let ty = f.ty.clone();
    let type_name = syn::Ident::new(&name.to_camel_case(), ident.span());
    let kw_name = syn::Ident::new(&name.to_shouty_snake_case(), ident.span());

    let desc = f.attrs.iter().fold(String::new(), |mut b, attr| {
        match attr.parse_meta().unwrap() {
            syn::Meta::NameValue(nv) => {
                if nv.ident == "doc" {
                    if let syn::Lit::Str(s) = nv.lit {
                        b = b + &s.value();
                    }
                }
            }
            _ => (),
        }
        b.trim().into()
    });

    let validation = match f
        .attrs
        .iter()
        .filter_map(|attr| {
            match attr.parse_meta().unwrap() {
                syn::Meta::List(nl) => {
                    //let nested = nl.nested.
                    if nl.ident == "check" {
                        Some(
                            nl.nested
                                .iter()
                                .map(|lit| match lit {
                                    syn::NestedMeta::Literal(Lit::Str(check_str)) => {
                                        Ok(check_str.value().parse()?)
                                    }
                                    elem => bail!("invalid check spec: {:?}", elem),
                                })
                                .next()
                                .unwrap_or_else(|| bail!("check attribute with out spec")),
                        )
                    } else {
                        None
                    }
                }
                _ => None,
            }
        })
        .next()
    {
        Some(Ok(v)) => Some(v),
        Some(Err(e)) => Err(e)?,
        None => None,
    };

    let conversion_type = (&ty).try_into().unwrap();

    Ok(SettingDef {
        type_name,
        kw_name,
        name,
        ty,
        desc,
        conversion_type,
        validation,
    })
}

pub fn gen_settings(mut f: syn::File) -> Fallible<TokenStream> {
    let mut sections = Vec::new();

    for item in &mut f.items {
        match item {
            syn::Item::Struct(s) => sections.push(parse_section(s)?),
            _ => {}
        }
    }

    let sections_q = sections.iter().map(|section| {
        let name = &section.name;
        let fields = section.items.iter().map(|setting| {
            let SettingDef {
                type_name,
                kw_name,
                ty,
                desc,
                name,
                conversion_type,
                validation
            } = setting;

            let validation_desc = setting.validation_desc();

            let val = quote!(val);

            let from_value = match conversion_type {
                ConversionType::Bool => quote! {
                    bool_from_value(#val)
                },
                _ => quote! {
                    Ok(serde_json::from_value(#val.clone())?)
                }
            };

            let to_value = match conversion_type {
                ConversionType::Bool => quote! {
                    bool_to_value(*item)
                },
                _ => quote! {
                    serde_json::json!(item)
                }
            };


            quote! {
                #[doc = #desc]
                pub struct #type_name;

                impl Setting for #type_name {
                    type Item = #ty;
                    const NAME : &'static str = #name;
                    const DESC : &'static str = #desc;
                    const VALIDATION_DESC : &'static str = #validation_desc;

                    // TODO: Add conversion
                    #[inline]
                    fn to_value(item : &#ty) -> Value {
                        #to_value
                    }

                    #[inline]
                    fn from_value(val :&Value) -> Result<#ty, Error> {
                        #from_value
                    }

                }

                unsafe impl Sync for #type_name {}

                pub(super) static #kw_name : #type_name = #type_name;
            }
        });

        let setting_list = section.items.iter().map(|setting| {
            &setting.kw_name
        });
        let n = section.items.len();

        quote! {
            pub mod #name {
                use super::*;

                #(#fields)*

                pub static SECTION_LIST : [&'static (dyn DynamicSetting + Sync); #n] = [#(&#setting_list),*];

                struct ListIter(usize);

                impl Iterator for ListIter {
                    type Item = &'static dyn DynamicSetting;

                    fn next(&mut self) -> Option<Self::Item> {
                        let pos = self.0;
                        if pos < SECTION_LIST.len() {
                            self.0 = pos + 1;
                            Some(SECTION_LIST[pos])
                        }
                        else {
                            None
                        }
                    }
                }

                pub fn list() -> impl Iterator<Item=&'static (dyn DynamicSetting + 'static)> {
                    ListIter(0)
                }
            }
        }
    });

    let from_name_rules = sections
        .iter()
        .map(|section| {
            let group_name = &section.name;

            section.items.iter().map(move |item| {
                let name = &item.name;
                let kw_name = &item.kw_name;

                quote! {
                    #name => {
                        Some(&#group_name::#kw_name)
                    }
                }
            })
        })
        .flatten();

    let key_names = sections
        .iter()
        .map(|section| section.items.iter().map(move |item| item.name.clone()))
        .flatten();

    Ok((quote! {
        #(#sections_q )*

        pub fn from_name(setting_name : &str) -> Option<&DynamicSetting> {
            match setting_name {
                #(#from_name_rules)*
                _ => None
            }
        }

        pub const NAMES : &[&str] = &[ #( #key_names ),* ];
    })
    .into())
}

fn lit_to_str(lit : &Expr) -> String {
    use quote::ToTokens;

    lit.clone().into_token_stream().to_string()
}


impl SettingDef {

    fn validation_desc(&self) -> String {
        match &self.validation {
            None => format!("{}", self.type_desc()),
            Some(Range { from: None, to: Some(to)}) => {
                format!("{} {} {}",self.type_desc(), to.as_lt_str(), lit_to_str(to.as_val()))
            }
            Some(Range { from: Some(from), to: None}) => {
                format!("{} {} {}",self.type_desc(), from.as_gt_str(), lit_to_str(from.as_val()))
            }

            Some(Range { from: Some(from), to: Some(to)}) => {
                format!("{} {} {} {} {}",lit_to_str(from.as_val()), from.as_lt_str(), self.type_desc(),  to.as_lt_str(), lit_to_str(to.as_val()))
            }
            _ => {
                eprintln!("range: {:?}", self.validation);
                unimplemented!()
            }
        }
    }

    fn type_desc(&self) -> &str {
        match self.conversion_type {
            ConversionType::Float => "float",
            ConversionType::Decimal => "decimal",
            ConversionType::String => "str",
            ConversionType::Bool => "bool",
            ConversionType::Nat => "int"
        }
    }

}

