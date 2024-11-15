use proc_macro::TokenStream;
use quote::quote;

#[proc_macro_derive(AbiExample)]
pub fn derive_abi_sample(_input: TokenStream) -> TokenStream {
    quote!().into()
}

#[proc_macro_derive(AbiEnumVisitor)]
pub fn derive_abi_enum_visitor(_input: TokenStream) -> TokenStream {
    quote!().into()
}

#[proc_macro_attribute]
pub fn frozen_abi(_attrs: TokenStream, item: TokenStream) -> TokenStream {
    item
}
