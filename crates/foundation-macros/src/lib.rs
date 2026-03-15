use proc_macro::TokenStream;
use syn::DeriveInput;
use syn::parse_macro_input;

mod docs;
mod expand;
mod output;
mod types;

#[proc_macro_derive(FoundationConfig, attributes(serde))]
pub fn derive_foundation_config(input: TokenStream) -> TokenStream {
    use quote::ToTokens;

    expand::derive_foundation_config_impl(parse_macro_input!(input as DeriveInput))
        .to_token_stream()
        .into()
}
