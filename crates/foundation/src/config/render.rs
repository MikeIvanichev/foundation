use std::io;

use foundation_types::config::ConfigSchema;
use foundation_types::config::Field;
use foundation_types::config::FieldKind;
use foundation_types::config::Path;
use foundation_types::config::Schema;

use crate::ServiceInfo;

/// Writes schema-derived config views for one env-prefix convention.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Renderer<'a> {
    env_prefix: &'a str,
}

impl<'a> Renderer<'a> {
    #[must_use]
    pub const fn new(env_prefix: &'a str) -> Self {
        Self { env_prefix }
    }

    #[must_use]
    pub const fn for_service(service_info: &'a ServiceInfo) -> Self {
        Self::new(service_info.env_prefix.as_str())
    }

    pub fn write_template<T>(self, out: &mut (impl io::Write + ?Sized)) -> io::Result<()>
    where
        T: ConfigSchema,
    {
        self.write::<T>(out, RenderMode::Template)
    }

    pub fn write_defaults<T>(self, out: &mut (impl io::Write + ?Sized)) -> io::Result<()>
    where
        T: ConfigSchema,
    {
        self.write::<T>(out, RenderMode::Defaults)
    }

    pub fn write_required<T>(self, out: &mut (impl io::Write + ?Sized)) -> io::Result<()>
    where
        T: ConfigSchema,
    {
        self.write::<T>(out, RenderMode::Required)
    }

    fn write<T>(self, out: &mut (impl io::Write + ?Sized), mode: RenderMode) -> io::Result<()>
    where
        T: ConfigSchema,
    {
        let mut path = Path::new();
        self.write_schema(out, &T::schema(), &mut path, RenderState::new(mode))?;
        out.write_all(b"\n")
    }

    fn write_schema(
        self,
        out: &mut (impl io::Write + ?Sized),
        schema: &Schema,
        path: &mut Path,
        state: RenderState,
    ) -> io::Result<bool> {
        let mut wrote_field = false;

        for field in &schema.fields {
            path.push(field.key);
            if self.field_would_render(field, path, state.parent_required, state.mode) {
                if wrote_field {
                    out.write_all(b"\n\n")?;
                }
                self.write_field(out, field, path, state)?;
                wrote_field = true;
            }
            let _ = path.pop();
        }

        Ok(wrote_field)
    }

    fn write_field(
        self,
        out: &mut (impl io::Write + ?Sized),
        field: &Field,
        path: &mut Path,
        state: RenderState,
    ) -> io::Result<()> {
        match &field.kind {
            FieldKind::Nested { schema } => self.write_section(out, field, schema, path, state),
            FieldKind::Leaf { .. } => self.write_leaf(out, field, path, state),
        }
    }

    fn write_section(
        self,
        out: &mut (impl io::Write + ?Sized),
        field: &Field,
        nested: &Schema,
        path: &mut Path,
        state: RenderState,
    ) -> io::Result<()> {
        let child_state = state.child(field.required);
        let has_body =
            self.schema_would_render(nested, path, child_state.parent_required, state.mode);

        write_doc_lines(out, state.indent, field.docs)?;
        write!(out, "{}{}:", " ".repeat(state.indent), field.key)?;

        if has_body {
            out.write_all(b"\n\n")?;
            self.write_schema(out, nested, path, child_state)?;
        }

        Ok(())
    }

    fn write_leaf(
        self,
        out: &mut (impl io::Write + ?Sized),
        field: &Field,
        path: &Path,
        state: RenderState,
    ) -> io::Result<()> {
        let required_here = match state.mode {
            RenderMode::Template => field.required,
            RenderMode::Defaults | RenderMode::Required => state.parent_required && field.required,
        };

        write_doc_lines(out, state.indent, field.docs)?;
        write_env_comment(out, state.indent, self.env_prefix, path)?;

        if required_here && matches!(state.mode, RenderMode::Template | RenderMode::Required) {
            return write!(out, "{}{}: <required>", " ".repeat(state.indent), field.key);
        }

        let default = field
            .default_yaml()
            .expect("leaf renderability checked before writing");

        if default.contains('\n') {
            write!(out, "{}{}:", " ".repeat(state.indent), field.key)?;
            let trimmed = default.trim_end();
            if !trimmed.is_empty() {
                out.write_all(b"\n")?;
                for (index, line) in trimmed.lines().enumerate() {
                    if index > 0 {
                        out.write_all(b"\n")?;
                    }
                    write!(out, "{}{}", " ".repeat(state.indent + 2), line)?;
                }
            }
        } else {
            write!(
                out,
                "{}{}: {}",
                " ".repeat(state.indent),
                field.key,
                default
            )?;
        }

        Ok(())
    }

    fn schema_would_render(
        self,
        schema: &Schema,
        path: &mut Path,
        parent_required: bool,
        mode: RenderMode,
    ) -> bool {
        schema.fields.iter().any(|field| {
            path.push(field.key);
            let would_render = self.field_would_render(field, path, parent_required, mode);
            let _ = path.pop();
            would_render
        })
    }

    fn field_would_render(
        self,
        field: &Field,
        path: &mut Path,
        parent_required: bool,
        mode: RenderMode,
    ) -> bool {
        let required_here = parent_required && field.required;

        match &field.kind {
            FieldKind::Leaf { .. } => match mode {
                RenderMode::Defaults => field.default_yaml().is_some(),
                RenderMode::Required => required_here,
                RenderMode::Template => field.required || field.default_yaml().is_some(),
            },
            FieldKind::Nested { schema } => {
                self.schema_would_render(schema, path, required_here, mode)
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RenderMode {
    Defaults,
    Required,
    Template,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct RenderState {
    indent: usize,
    parent_required: bool,
    mode: RenderMode,
}

impl RenderState {
    const fn new(mode: RenderMode) -> Self {
        Self {
            indent: 0,
            parent_required: true,
            mode,
        }
    }

    const fn child(self, field_required: bool) -> Self {
        Self {
            indent: self.indent + 2,
            parent_required: self.parent_required && field_required,
            mode: self.mode,
        }
    }
}

fn write_doc_lines(
    out: &mut (impl io::Write + ?Sized),
    indent: usize,
    docs: &[&str],
) -> io::Result<()> {
    for doc in docs {
        writeln!(out, "{}# {}", " ".repeat(indent), doc)?;
    }
    Ok(())
}

fn write_env_comment(
    out: &mut (impl io::Write + ?Sized),
    indent: usize,
    env_prefix: &str,
    path: &Path,
) -> io::Result<()> {
    if !env_prefix.is_empty() || !path.is_empty() {
        write!(out, "{}# env: ", " ".repeat(indent))?;
        path.write_env_with_prefix(out, env_prefix)?;
        out.write_all(b"\n")?;
    }
    Ok(())
}
