#![recursion_limit = "128"]
extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn::parse_macro_input;

mod settings;

#[proc_macro]
pub fn gen_settings(input: TokenStream) -> TokenStream {
    let f: syn::File = parse_macro_input!(input as syn::File);
    match settings::gen_settings(f) {
        Ok(v) => v.into(),
        Err(e) => {
            let err_msg = format!("{}", e);
            (quote! {
                compile_error!(#err_msg);
            })
            .into()
        }
    }
}
