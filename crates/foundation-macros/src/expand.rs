use proc_macro2::Span;
use quote::quote;
use serde_derive_internals::Ctxt;
use serde_derive_internals::Derive;
use serde_derive_internals::ast;
use serde_derive_internals::attr;
use std::collections::BTreeMap;
use syn::DeriveInput;
use syn::ExprPath;
use syn::Member;
use syn::Type;
use syn::spanned::Spanned;

use crate::docs::extract_docs;
use crate::output::DeriveOutput;
use crate::output::ErrorStore;
use crate::types::classify_type;
use crate::types::extract_single_type_argument;

pub(crate) fn derive_foundation_config_impl(input: DeriveInput) -> DeriveOutput {
    let ident = input.ident.clone();
    let generics = input.generics.clone();
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    let mut errors = ErrorStore::default();
    let cx = Ctxt::new();
    let parsed = ast::Container::from_ast(&cx, &input, Derive::Deserialize);
    if let Err(error) = cx.check() {
        errors.push(error);
    }

    let Some(container) = parsed else {
        return DeriveOutput {
            tokens: None,
            errors: errors.into_vec(),
        };
    };
    let fields = match &container.data {
        ast::Data::Struct(ast::Style::Struct, fields) => fields,
        ast::Data::Struct(_, _) => {
            errors.push(syn::Error::new(
                ident.span(),
                "FoundationConfig only supports structs with named fields",
            ));
            return DeriveOutput {
                tokens: None,
                errors: errors.into_vec(),
            };
        }
        ast::Data::Enum(_) => {
            errors.push(syn::Error::new(
                ident.span(),
                "FoundationConfig only supports structs",
            ));
            return DeriveOutput {
                tokens: None,
                errors: errors.into_vec(),
            };
        }
    };
    let mut field_stmts = Vec::new();
    let mut direct_keys = BTreeMap::<String, Span>::new();

    for field in fields {
        let Member::Named(field_ident) = &field.member else {
            errors.push(syn::Error::new(
                field.original.span(),
                "FoundationConfig only supports named fields",
            ));
            continue;
        };

        if field.attrs.skip_deserializing() {
            continue;
        }

        let field_name = field_ident.to_string();
        let key = field.attrs.name().deserialize_name().to_owned();
        let kind = classify_type(field.ty);
        let nested_ty = nested_type(field.ty, kind);
        let flatten = field.attrs.flatten();

        if !flatten && let Some(existing_span) = direct_keys.insert(key.clone(), field_ident.span())
        {
            errors.push(syn::Error::new(
                field_ident.span(),
                format!("duplicate config key `{key}`"),
            ));
            errors.push(syn::Error::new(
                existing_span,
                format!("`{key}` first defined here"),
            ));
        }

        if flatten && nested_ty.is_none() {
            errors.push(syn::Error::new(
                field.ty.span(),
                "#[serde(flatten)] is only supported on nested config structs",
            ));
        }

        let default = match field.attrs.default() {
            attr::Default::None => FieldDefault::None,
            attr::Default::Default => FieldDefault::Trait,
            attr::Default::Path(path) => FieldDefault::Path(path.clone()),
        };

        let has_default = !matches!(default, FieldDefault::None);
        let nested = nested_ty.is_some();
        let required = !has_default && !kind.optional;
        match schema_field_stmt(&SchemaFieldStmtArgs {
            rust_name: &field_name,
            key: &key,
            docs: &extract_docs(&field.original.attrs),
            ty: field.ty,
            nested,
            flatten,
            default: &default,
            required,
            nested_ty: nested_ty.as_ref(),
            span: field.ty.span(),
        }) {
            Ok(stmt) => field_stmts.push(stmt),
            Err(error) => errors.push(error),
        }
    }

    let tokens = quote! {
        impl #impl_generics ::foundation_types::config::ConfigSchema for #ident #ty_generics #where_clause {
            fn schema() -> ::foundation_types::config::Schema {
                let mut fields = ::std::vec::Vec::new();
                #(#field_stmts)*
                ::foundation_types::config::Schema { fields }
            }
        }
    };

    DeriveOutput {
        tokens: Some(tokens),
        errors: errors.into_vec(),
    }
}

#[derive(Clone)]
enum FieldDefault {
    None,
    Trait,
    Path(ExprPath),
}

struct SchemaFieldStmtArgs<'a> {
    rust_name: &'a str,
    key: &'a str,
    docs: &'a [String],
    ty: &'a Type,
    nested: bool,
    flatten: bool,
    default: &'a FieldDefault,
    required: bool,
    nested_ty: Option<&'a Type>,
    span: Span,
}

fn nested_type(ty: &Type, kind: crate::types::FieldTypeInfo) -> Option<Type> {
    if !kind.nested {
        return None;
    }

    if kind.optional {
        return extract_single_type_argument(ty).cloned();
    }

    Some(ty.clone())
}

fn schema_field_stmt(args: &SchemaFieldStmtArgs<'_>) -> syn::Result<proc_macro2::TokenStream> {
    let docs = args.docs.iter().map(|doc| quote! { #doc });
    let key = args.key;
    let required = args.required;

    if args.flatten {
        let nested_ty = args.nested_ty.ok_or_else(|| {
            syn::Error::new(
                args.span,
                "#[serde(flatten)] requires nested config metadata",
            )
        })?;
        return Ok(quote! {
            fields.extend(
                <#nested_ty as ::foundation_types::config::ConfigSchema>::schema()
                    .fields
                    .into_iter()
                    .map(|field| field.under_required_parent(#required))
            );
        });
    }

    let kind = if args.nested {
        let nested_ty = args.nested_ty.ok_or_else(|| {
            syn::Error::new(args.span, "nested config field is missing schema metadata")
        })?;
        quote! {
            ::foundation_types::config::FieldKind::Nested {
                schema: Box::new(<#nested_ty as ::foundation_types::config::ConfigSchema>::schema())
            }
        }
    } else {
        let default = field_default_value_tokens(args.rust_name, args.ty, args.default);
        quote! {
            ::foundation_types::config::FieldKind::Leaf { default_yaml: #default }
        }
    };

    Ok(quote! {
        fields.push(::foundation_types::config::Field {
            key: #key,
            docs: &[#(#docs),*],
            required: #required,
            kind: #kind,
        });
    })
}

fn field_default_value_tokens(
    rust_name: &str,
    ty: &Type,
    default: &FieldDefault,
) -> proc_macro2::TokenStream {
    let context = format!("failed to serialize default for `{rust_name}`");

    match default {
        FieldDefault::Path(path) => quote! {{
            ::core::option::Option::Some(
                ::serde_saphyr::to_string(&{ #path() })
                    .map(|yaml| yaml.trim_end().to_owned())
                    .expect(#context),
            )
        }},
        FieldDefault::Trait => quote! {{
            ::core::option::Option::Some(
                ::serde_saphyr::to_string(
                    &<#ty as ::core::default::Default>::default()
                )
                .map(|yaml| yaml.trim_end().to_owned())
                .expect(#context),
            )
        }},
        FieldDefault::None => quote! { ::core::option::Option::None },
    }
}

#[cfg(test)]
mod tests {
    use quote::ToTokens;
    use syn::parse_quote;

    use super::derive_foundation_config_impl;

    #[test]
    fn derive_emits_config_impl() {
        let input: syn::DeriveInput = parse_quote! {
            struct Example {
                #[serde(default)]
                enabled: bool,
            }
        };

        let output = derive_foundation_config_impl(input)
            .to_token_stream()
            .to_string();

        assert!(output.contains("config :: ConfigSchema"));
    }
}
