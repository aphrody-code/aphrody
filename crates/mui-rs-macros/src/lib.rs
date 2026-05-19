extern crate proc_macro;
use proc_macro::TokenStream;

/// Mark a struct as a M3 component (future: auto-derive accesskit Node).
#[proc_macro_attribute]
pub fn m3_component(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}

/// Derive M3 token conversion traits.
#[proc_macro_derive(M3Token)]
pub fn derive_m3_token(_item: TokenStream) -> TokenStream {
    TokenStream::new()
}
