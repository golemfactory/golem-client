extern crate proc_macro;
use proc_macro::TokenStream;

#[proc_macro_attribute]
pub fn wamp_interface(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}

#[proc_macro_attribute]
pub fn wamp(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}
