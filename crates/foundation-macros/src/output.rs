use proc_macro2::TokenStream;
use quote::ToTokens;
use quote::TokenStreamExt;

#[derive(Default)]
pub(crate) struct ErrorStore {
    errors: Vec<syn::Error>,
}

impl ErrorStore {
    pub(crate) fn push(&mut self, error: syn::Error) {
        self.errors.push(error);
    }

    pub(crate) fn into_vec(self) -> Vec<syn::Error> {
        self.errors
    }
}

pub(crate) struct DeriveOutput {
    pub(crate) tokens: Option<TokenStream>,
    pub(crate) errors: Vec<syn::Error>,
}

impl ToTokens for DeriveOutput {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        if let Some(output) = &self.tokens {
            tokens.append_all(output.clone());
        }

        for error in &self.errors {
            tokens.append_all(error.to_compile_error());
        }
    }
}
