use std::fmt::Write as _;

use foundation_types::config::ConfigSchema;
use foundation_types::config::Field;
use foundation_types::config::FieldKind;
use foundation_types::config::Path;
use foundation_types::config::Schema;

use crate::ServiceInfo;

use super::DEFAULT_ENV_PREFIX;

/// A computed environment variable name.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EnvName(String);

impl EnvName {
    #[must_use]
    pub fn new(prefix: &str, path: &Path) -> Self {
        let mut name = prefix.to_ascii_uppercase();

        let mut keys = path.keys().peekable();
        if !name.is_empty() && keys.peek().is_some() {
            name.push_str("__");
        }

        let mut first = true;
        for key in keys {
            if !first {
                name.push_str("__");
            }
            name.push_str(&key.to_ascii_uppercase());
            first = false;
        }

        Self(name)
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Rendering mode for config output.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RenderMode {
    Defaults,
    Required,
    Template,
}

/// Required-field metadata for CLI output.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RequiredField {
    pub path: Path,
    pub env: EnvName,
    pub docs: &'static [&'static str],
}

/// Renders schema-derived config views for one env-prefix convention.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Render<'a> {
    env_prefix: &'a str,
}

impl Default for Render<'static> {
    fn default() -> Self {
        Self::new(DEFAULT_ENV_PREFIX)
    }
}

impl<'a> Render<'a> {
    #[must_use]
    pub const fn new(env_prefix: &'a str) -> Self {
        Self { env_prefix }
    }

    #[must_use]
    pub fn for_service(service_info: &'a ServiceInfo) -> Self {
        Self::new(&service_info.config_env_prefix)
    }

    #[must_use]
    pub fn defaults<T>(self) -> String
    where
        T: ConfigSchema,
    {
        self.render::<T>(RenderMode::Defaults)
    }

    #[must_use]
    pub fn template<T>(self) -> String
    where
        T: ConfigSchema,
    {
        self.render::<T>(RenderMode::Template)
    }

    #[must_use]
    pub fn required_fields<T>(self) -> Vec<RequiredField>
    where
        T: ConfigSchema,
    {
        RequiredFieldCollector::new(self.env_prefix).collect::<T>()
    }

    #[must_use]
    pub fn required<T>(self) -> String
    where
        T: ConfigSchema,
    {
        self.render::<T>(RenderMode::Required)
    }

    fn render<T>(self, mode: RenderMode) -> String
    where
        T: ConfigSchema,
    {
        let mut rendered =
            YamlRenderer::new(mode, self.env_prefix).render_schema(&T::schema(), &Path::new(), 0);
        rendered.push('\n');
        rendered
    }
}

// ---------------------------------------------------------------------------
// Required field collection
// ---------------------------------------------------------------------------

struct RequiredFieldCollector<'a> {
    env_prefix: &'a str,
    fields: Vec<RequiredField>,
}

impl<'a> RequiredFieldCollector<'a> {
    fn new(env_prefix: &'a str) -> Self {
        Self {
            env_prefix,
            fields: Vec::new(),
        }
    }

    fn collect<T>(mut self) -> Vec<RequiredField>
    where
        T: ConfigSchema,
    {
        self.collect_schema(&T::schema(), &Path::new(), true);
        self.fields
    }

    fn collect_schema(&mut self, schema: &Schema, parent: &Path, parent_required: bool) {
        for field in &schema.fields {
            let path = parent.with_key(field.key);
            let required_here = parent_required && field.required;

            match &field.kind {
                FieldKind::Leaf { .. } => {
                    if required_here {
                        self.fields.push(RequiredField {
                            path: path.clone(),
                            env: EnvName::new(self.env_prefix, &path),
                            docs: field.docs,
                        });
                    }
                }
                FieldKind::Nested { schema } => {
                    self.collect_schema(schema, &path, required_here);
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// YAML template/defaults/required rendering
// ---------------------------------------------------------------------------

struct YamlRenderer<'a> {
    mode: RenderMode,
    env_prefix: &'a str,
}

impl<'a> YamlRenderer<'a> {
    fn new(mode: RenderMode, env_prefix: &'a str) -> Self {
        Self { mode, env_prefix }
    }

    fn render_schema(&self, schema: &Schema, parent: &Path, indent: usize) -> String {
        self.render_fields(schema, parent, indent, true)
            .join("\n\n")
    }

    fn render_fields(
        &self,
        schema: &Schema,
        parent: &Path,
        indent: usize,
        parent_required: bool,
    ) -> Vec<String> {
        let mut sections = Vec::new();

        for field in &schema.fields {
            let path = parent.with_key(field.key);
            if let Some(section) = self.render_field(field, &path, indent, parent_required) {
                sections.push(section);
            }
        }

        sections
    }

    fn render_field(
        &self,
        field: &Field,
        path: &Path,
        indent: usize,
        parent_required: bool,
    ) -> Option<String> {
        match &field.kind {
            FieldKind::Nested { schema } => {
                self.render_section(field, schema, path, indent, parent_required)
            }
            FieldKind::Leaf { .. } => self.render_leaf(field, path, indent, parent_required),
        }
    }

    fn render_section(
        &self,
        field: &Field,
        nested: &Schema,
        path: &Path,
        indent: usize,
        parent_required: bool,
    ) -> Option<String> {
        let body = self
            .render_fields(nested, path, indent + 2, parent_required && field.required)
            .join("\n\n");
        if matches!(self.mode, RenderMode::Defaults | RenderMode::Required) && body.is_empty() {
            return None;
        }

        let mut out = render_doc_lines(field.docs, indent);
        let _ = write!(out, "{}{}:", " ".repeat(indent), field.key);
        if !body.is_empty() {
            out.push_str("\n\n");
            out.push_str(&body);
        }
        Some(out)
    }

    fn render_leaf(
        &self,
        field: &Field,
        path: &Path,
        indent: usize,
        parent_required: bool,
    ) -> Option<String> {
        let required_here = parent_required && field.required;

        match self.mode {
            RenderMode::Defaults if field.default_yaml().is_none() => return None,
            RenderMode::Required if !required_here => return None,
            _ => {}
        }

        let mut out = String::new();
        write_doc_lines_into(&mut out, indent, field.docs);
        write_env_comment(&mut out, indent, self.env_prefix, path);

        if required_here && matches!(self.mode, RenderMode::Template | RenderMode::Required) {
            let _ = write!(out, "{}{}: <required>", " ".repeat(indent), field.key);
            return Some(out);
        }

        let default = field.default_yaml()?;

        if default.contains('\n') {
            let _ = write!(out, "{}{}:", " ".repeat(indent), field.key);
            let trimmed = default.trim_end();
            if !trimmed.is_empty() {
                out.push('\n');
                for (index, line) in trimmed.lines().enumerate() {
                    if index > 0 {
                        out.push('\n');
                    }
                    let _ = write!(out, "{}{}", " ".repeat(indent + 2), line);
                }
            }
        } else {
            let _ = write!(out, "{}{}: {}", " ".repeat(indent), field.key, default);
        }

        Some(out)
    }
}

fn render_doc_lines(docs: &[&'static str], indent: usize) -> String {
    let mut out = String::new();
    write_doc_lines_into(&mut out, indent, docs);
    out
}

fn write_doc_lines_into(out: &mut String, indent: usize, docs: &[&str]) {
    for doc in docs {
        let _ = writeln!(out, "{}# {}", " ".repeat(indent), doc);
    }
}

fn write_env_comment(out: &mut String, indent: usize, env_prefix: &str, path: &Path) {
    let env = EnvName::new(env_prefix, path);
    if !env.as_str().is_empty() {
        let _ = writeln!(out, "{}# env: {}", " ".repeat(indent), env.as_str());
    }
}
