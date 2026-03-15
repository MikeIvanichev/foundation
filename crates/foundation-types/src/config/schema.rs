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

impl Field {
    /// Returns the rendered YAML default for a leaf field.
    #[must_use]
    pub fn default_yaml(&self) -> Option<&str> {
        match &self.kind {
            FieldKind::Leaf { default_yaml } => default_yaml.as_deref(),
            FieldKind::Nested { .. } => None,
        }
    }

    /// Returns nested schema metadata when the field is structured.
    #[must_use]
    pub fn nested(&self) -> Option<&Schema> {
        match &self.kind {
            FieldKind::Leaf { .. } => None,
            FieldKind::Nested { schema } => Some(schema),
        }
    }

    /// Applies a parent required-state to this field.
    #[must_use]
    pub fn under_required_parent(mut self, parent_required: bool) -> Self {
        self.required = self.required && parent_required;
        self
    }
}
