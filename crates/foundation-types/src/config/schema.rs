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

impl Schema {
    /// Creates a builder that validates schema invariants while fields are
    /// added.
    #[must_use]
    pub fn builder() -> SchemaBuilder {
        SchemaBuilder::default()
    }
}

/// Builder for [`Schema`] values.
#[derive(Debug, Default)]
pub struct SchemaBuilder {
    fields: Vec<Field>,
}

impl SchemaBuilder {
    /// Adds one field to the schema.
    ///
    /// # Panics
    ///
    /// Panics when another field with the same key already exists at this
    /// schema level.
    pub fn push(&mut self, field: Field) {
        if self.fields.iter().any(|existing| existing.key == field.key) {
            panic!("duplicate config key `{}`", field.key);
        }

        self.fields.push(field);
    }

    /// Adds a flattened child schema.
    ///
    /// # Panics
    ///
    /// Panics when any flattened field duplicates a sibling key.
    pub fn extend_flattened(&mut self, schema: Schema, required: bool) {
        for mut field in schema.fields {
            field.required = field.required && required;
            self.push(field);
        }
    }

    /// Finishes the schema.
    #[must_use]
    pub fn build(self) -> Schema {
        Schema {
            fields: self.fields,
        }
    }
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
