use std::io;

/// Full unique identifier for a configuration variable.
#[derive(Clone, Debug, Default, Eq, Ord, PartialEq, PartialOrd, Hash)]
pub struct Path {
    segments: Vec<String>,
}

impl Path {
    /// Creates an empty path.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            segments: Vec::new(),
        }
    }

    /// Appends one segment and returns the path.
    #[must_use]
    pub fn join(mut self, segment: impl Into<String>) -> Self {
        self.push(segment);
        self
    }

    /// Appends one segment in place.
    pub fn push(&mut self, segment: impl Into<String>) {
        self.segments.push(segment.into());
    }

    /// Removes and returns the final segment.
    pub fn pop(&mut self) -> Option<String> {
        self.segments.pop()
    }

    /// Returns whether the path has no segments.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.segments.is_empty()
    }

    /// Iterates path segments in order.
    pub fn iter(&self) -> impl Iterator<Item = &str> + '_ {
        self.segments.iter().map(String::as_str)
    }

    /// Writes the path in dotted config notation (`.` separator).
    pub fn write_dotted(&self, out: &mut (impl io::Write + ?Sized)) -> io::Result<()> {
        for (index, segment) in self.iter().enumerate() {
            if index > 0 {
                out.write_all(b".")?;
            }
            out.write_all(segment.as_bytes())?;
        }

        Ok(())
    }

    /// Writes the path in environment-variable notation (`__` separator).
    pub fn write_env(&self, out: &mut (impl io::Write + ?Sized)) -> io::Result<()> {
        self.write_env_with_prefix(out, "")
    }

    /// Writes the path in environment-variable notation (`__` separator) with
    /// an explicit prefix.
    pub fn write_env_with_prefix(
        &self,
        out: &mut (impl io::Write + ?Sized),
        prefix: &str,
    ) -> io::Result<()> {
        write_ascii_uppercase(out, prefix)?;

        for (index, segment) in self.iter().enumerate() {
            if index > 0 || !prefix.is_empty() {
                out.write_all(b"__")?;
            }
            write_ascii_uppercase(out, segment)?;
        }

        Ok(())
    }
}

fn write_ascii_uppercase(out: &mut (impl io::Write + ?Sized), value: &str) -> io::Result<()> {
    for byte in value.bytes() {
        out.write_all(&[byte.to_ascii_uppercase()])?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::Path;

    #[test]
    fn writes_dotted_paths() {
        let path = Path::new().join("otel").join("endpoint");

        let mut out = Vec::new();
        path.write_dotted(&mut out).expect("write path");
        let out = String::from_utf8(out).expect("path is UTF-8");

        assert_eq!(out, "otel.endpoint");
        assert_eq!(path.iter().collect::<Vec<_>>(), vec!["otel", "endpoint"]);
    }

    #[test]
    fn writes_env_paths() {
        let path = Path::new().join("otel").join("endpoint");

        let mut without_prefix = Vec::new();
        path.write_env(&mut without_prefix).expect("write path");
        let without_prefix = String::from_utf8(without_prefix).expect("path is UTF-8");

        let mut with_prefix = Vec::new();
        path.write_env_with_prefix(&mut with_prefix, "app")
            .expect("write path");
        let with_prefix = String::from_utf8(with_prefix).expect("path is UTF-8");

        assert_eq!(without_prefix, "OTEL__ENDPOINT");
        assert_eq!(with_prefix, "APP__OTEL__ENDPOINT");
    }

    #[test]
    fn pushes_and_pops_segments() {
        let mut path = Path::new().join("otel").join("endpoint");

        path.push("url");

        let mut out = Vec::new();
        path.write_dotted(&mut out).expect("write path");
        let out = String::from_utf8(out).expect("path is UTF-8");
        assert_eq!(out, "otel.endpoint.url");

        assert_eq!(path.pop().as_deref(), Some("url"));

        let mut out = Vec::new();
        path.write_dotted(&mut out).expect("write path");
        let out = String::from_utf8(out).expect("path is UTF-8");
        assert_eq!(out, "otel.endpoint");
    }
}
