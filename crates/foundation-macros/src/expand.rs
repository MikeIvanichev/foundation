use std::collections::BTreeMap;

use proc_macro2::Span;
use quote::quote;
use serde_derive_internals::Ctxt;
use serde_derive_internals::Derive;
use serde_derive_internals::ast;
use serde_derive_internals::attr;
use syn::DeriveInput;
use syn::ExprPath;
use syn::Member;
use syn::Type;
use syn::spanned::Spanned;

use crate::docs::extract_docs;
use crate::output::DeriveOutput;
use crate::types::FieldTypeInfo;
use crate::types::classify_type;
use crate::types::extract_single_type_argument;

pub(crate) fn derive_foundation_config_impl(input: DeriveInput) -> DeriveOutput {
    let ident = input.ident.clone();
    let generics = input.generics.clone();
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    let mut errors = Vec::new();
    let cx = Ctxt::new();
    let parsed = ast::Container::from_ast(&cx, &input, Derive::Deserialize);
    if let Err(error) = cx.check() {
        errors.push(error);
    }

    let Some(container) = parsed else {
        return DeriveOutput {
            tokens: None,
            errors,
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
                errors,
            };
        }
        ast::Data::Enum(_) => {
            errors.push(syn::Error::new(
                ident.span(),
                "FoundationConfig only supports structs",
            ));
            return DeriveOutput {
                tokens: None,
                errors,
            };
        }
    };
    let mut field_tokens = Vec::new();
    let mut direct_field_keys = BTreeMap::<String, Span>::new();

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
        let type_info = classify_type(field.ty);
        let nested_config_ty = nested_config_type(field.ty, type_info);
        let flatten = field.attrs.flatten();

        if !flatten
            && let Some(existing_span) = direct_field_keys.insert(key.clone(), field_ident.span())
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

        if flatten && nested_config_ty.is_none() {
            errors.push(syn::Error::new(
                field.ty.span(),
                "#[serde(flatten)] is only supported on nested config structs",
            ));
        }

        let default_source = match field.attrs.default() {
            attr::Default::None => DefaultSource::None,
            attr::Default::Default => DefaultSource::Trait,
            attr::Default::Path(path) => DefaultSource::Path(path.clone()),
        };

        let has_default = !matches!(default_source, DefaultSource::None);
        let required = !has_default && !type_info.is_optional;
        match schema_field_tokens(&SchemaFieldTokens {
            rust_name: &field_name,
            key: &key,
            docs: &extract_docs(&field.original.attrs),
            ty: field.ty,
            flatten,
            default_source: &default_source,
            required,
            nested_config_ty: nested_config_ty.as_ref(),
            span: field.ty.span(),
        }) {
            Ok(tokens) => field_tokens.push(tokens),
            Err(error) => errors.push(error),
        }
    }

    let tokens = quote! {
        impl #impl_generics ::foundation_types::config::ConfigSchema for #ident #ty_generics #where_clause {
            fn schema() -> ::foundation_types::config::Schema {
                let mut fields = ::foundation_types::config::Schema::builder();
                #(#field_tokens)*
                fields.build()
            }
        }
    };

    DeriveOutput {
        tokens: Some(tokens),
        errors,
    }
}

#[derive(Clone)]
enum DefaultSource {
    None,
    Trait,
    Path(ExprPath),
}

struct SchemaFieldTokens<'a> {
    rust_name: &'a str,
    key: &'a str,
    docs: &'a [String],
    ty: &'a Type,
    flatten: bool,
    default_source: &'a DefaultSource,
    required: bool,
    nested_config_ty: Option<&'a Type>,
    span: Span,
}

fn nested_config_type(ty: &Type, type_info: FieldTypeInfo) -> Option<Type> {
    if !type_info.is_nested {
        return None;
    }

    if type_info.is_optional {
        return extract_single_type_argument(ty).cloned();
    }

    Some(ty.clone())
}

fn schema_field_tokens(args: &SchemaFieldTokens<'_>) -> syn::Result<proc_macro2::TokenStream> {
    let docs = args.docs.iter().map(|doc| quote! { #doc });
    let key = args.key;
    let required = args.required;

    if args.flatten {
        let nested_config_ty = args.nested_config_ty.ok_or_else(|| {
            syn::Error::new(
                args.span,
                "#[serde(flatten)] requires nested config metadata",
            )
        })?;
        return Ok(quote! {
            fields.extend_flattened(
                <#nested_config_ty as ::foundation_types::config::ConfigSchema>::schema()
                ,
                #required
            );
        });
    }

    let kind = if let Some(nested_config_ty) = args.nested_config_ty {
        quote! {
            ::foundation_types::config::FieldKind::Nested {
                schema: Box::new(<#nested_config_ty as ::foundation_types::config::ConfigSchema>::schema())
            }
        }
    } else {
        let default = default_yaml_tokens(args.rust_name, args.ty, args.default_source);
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

fn default_yaml_tokens(
    rust_name: &str,
    ty: &Type,
    default_source: &DefaultSource,
) -> proc_macro2::TokenStream {
    let context = format!("failed to serialize default for `{rust_name}`");

    match default_source {
        DefaultSource::Path(path) => quote! {{
            ::core::option::Option::Some(
                ::serde_saphyr::to_string(&{ #path() })
                    .map(|yaml| yaml.trim_end().to_owned())
                    .expect(#context),
            )
        }},
        DefaultSource::Trait => quote! {{
            ::core::option::Option::Some(
                ::serde_saphyr::to_string(
                    &<#ty as ::core::default::Default>::default()
                )
                .map(|yaml| yaml.trim_end().to_owned())
                .expect(#context),
            )
        }},
        DefaultSource::None => quote! { ::core::option::Option::None },
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
