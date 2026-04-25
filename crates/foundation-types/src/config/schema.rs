/// Static documentation metadata for a configuration type.
pub trait ConfigSchema {
    /// Returns documentation metadata for the type.
    fn schema() -> Schema;
}

/// Structured documentation metadata for a configuration type.
#[derive(Clone, Debug, PartialEq)]
pub struct Schema {
    /// Fields defined on the type.
    pub fields: Vec<Field>,
}

/// Metadata for one documented configuration field.
#[derive(Clone, Debug, PartialEq)]
pub struct Field {
    /// Serialized configuration key.
    pub key: &'static str,
    /// Field documentation lines.
    pub docs: &'static [&'static str],
    /// Whether the field or nested subtree must be supplied by the caller.
    pub required: bool,
    /// Field-specific metadata.
    pub kind: FieldKind,
}

impl Field {
    /// Returns the YAML default for this field when it is a leaf.
    #[must_use]
    pub fn default_yaml(&self) -> Option<&str> {
        match &self.kind {
            FieldKind::Leaf { default_yaml } => default_yaml.as_deref(),
            FieldKind::Nested { .. } => None,
        }
    }

    /// Returns the child schema for this field when it is nested.
    #[must_use]
    pub fn nested_schema(&self) -> Option<&Schema> {
        match &self.kind {
            FieldKind::Leaf { .. } => None,
            FieldKind::Nested { schema } => Some(schema),
        }
    }
}

/// Variant-specific metadata for one documented configuration field.
#[derive(Clone, Debug, PartialEq)]
pub enum FieldKind {
    /// Metadata for a scalar or collection value.
    Leaf {
        /// YAML fragment for the field default, if one exists.
        default_yaml: Option<String>,
    },
    /// Metadata for a structured nested value.
    Nested {
        /// Nested documentation metadata for structured values.
        schema: Box<Schema>,
    },
}
