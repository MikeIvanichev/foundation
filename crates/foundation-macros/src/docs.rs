use syn::Expr;
use syn::Lit;
use syn::Meta;
use syn::MetaNameValue;

pub(crate) fn extract_docs(attrs: &[syn::Attribute]) -> Vec<String> {
    attrs
        .iter()
        .filter(|attr| attr.path().is_ident("doc"))
        .filter_map(|attr| match &attr.meta {
            Meta::NameValue(MetaNameValue {
                value:
                    Expr::Lit(syn::ExprLit {
                        lit: Lit::Str(value),
                        ..
                    }),
                ..
            }) => Some(value.value().trim().to_owned()),
            _ => None,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::extract_docs;
    use syn::parse_quote;

    #[test]
    fn extracts_doc_comments() {
        let field: syn::Field = parse_quote! {
            #[doc = " First line "]
            #[doc = "Second line"]
            name: String
        };

        assert_eq!(
            extract_docs(&field.attrs),
            vec!["First line", "Second line"]
        );
    }
}
