extern crate proc_macro;
use proc_macro::TokenStream;

#[proc_macro_attribute]
pub fn wamp_interface(attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}

#[proc_macro_attribute]
pub fn wamp(attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}
