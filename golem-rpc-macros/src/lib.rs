#![recursion_limit="128"]

extern crate proc_macro;
use heck::{CamelCase, SnakeCase, ShoutySnakeCase};
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, File, Type};
use std::convert::{TryFrom, TryInto};

struct SettingSection {
    name: syn::Ident,
    items: Vec<SettingDef>,
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

impl TryFrom<&syn::Type> for ConversionType {
    type Error = failure::Error;

    fn try_from(value: &syn::Type) -> Result<Self, Self::Error> {
        match value {
            syn::Type::Path(p) => {
                if p.path.is_ident("usize") {
                    return Ok(ConversionType::Nat)
                }
                if p.path.is_ident("bool") {
                    return Ok(ConversionType::Bool)
                }
                if p.path.is_ident("String") {
                    return Ok(ConversionType::String)
                }
                if p.path.is_ident("BigDecimal") {
                    return Ok(ConversionType::Decimal)
                }
                if p.path.is_ident("f64") {
                    return Ok(ConversionType::Float)
                }
                Err(failure::err_msg(format!("Unsupported type: {:?}", p)))
            }
            v => Err(failure::err_msg(format!("Unsupported type: {:?}", v)))
        }
    }
}

struct SettingDef {
    type_name: syn::Ident,
    kw_name : syn::Ident,
    ty: syn::Type,
    desc: String,
    name: String,
    conversion_type : ConversionType,
}

#[proc_macro]
pub fn gen_settings(input: TokenStream) -> TokenStream {
    let mut f: syn::File = parse_macro_input!(input as File);

    let mut sections = Vec::new();

    for item in &mut f.items {
        match item {
            syn::Item::Struct(s) => sections.push(parse_section(s)),
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
                ..
            } = setting;

            quote! {
                pub struct #type_name;

                impl Setting for #type_name {
                    type Item = #ty;
                    const NAME : &'static str = #name;
                    const DESC : &'static str = #desc;

                    // TODO: Add conversion
                    #[inline]
                    fn to_value(item : &#ty) -> Value {
                        serde_json::json!(item)
                    }

                    #[inline]
                    fn from_value(val :&Value) -> Result<#ty, Error> {
                        Ok(serde_json::from_value(val.clone())?)
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

    let from_name_rules = sections.iter().map(|section| {
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
    }).flatten();

    (quote! {
        #(#sections_q )*

        pub fn from_name(setting_name : &str) -> Option<&DynamicSetting> {
            match setting_name {
                #(#from_name_rules)*
                _ => None
            }
        }
    }).into()
}

fn parse_section(s: &syn::ItemStruct) -> SettingSection {
    let name = syn::Ident::new(&s.ident.to_string().to_snake_case(), s.ident.span());
    let items = s.fields.iter().map(|f| parse_section_field(f)).collect();

    SettingSection { name, items }
}

fn parse_section_field(f: &syn::Field) -> SettingDef {
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

    let conversion_type = (&ty).try_into().unwrap();

    SettingDef {
        type_name,
        kw_name,
        name,
        ty,
        desc,
        conversion_type
    }
}
