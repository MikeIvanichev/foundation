use core::fmt;

/// Logical dotted configuration path.
#[derive(Clone, Debug, Default, Eq, Ord, PartialEq, PartialOrd, Hash)]
pub struct Path {
    segments: Vec<String>,
}

impl Path {
    /// Creates an empty path.
    #[must_use]
    pub fn new() -> Self {
        Self {
            segments: Vec::new(),
        }
    }

    /// Appends a key segment.
    #[must_use]
    pub fn with_key(&self, key: impl Into<String>) -> Self {
        let mut segments = self.segments.clone();
        segments.push(key.into());
        Self { segments }
    }

    /// Iterates the path segments in order.
    pub fn keys(&self) -> impl Iterator<Item = &str> + '_ {
        self.segments.iter().map(String::as_str)
    }
}

impl fmt::Display for Path {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.segments.join("."))
    }
}

#[cfg(test)]
mod tests {
    use super::Path;

    #[test]
    fn formats_paths() {
        let path = Path::new().with_key("otel").with_key("endpoint");
        assert_eq!(path.to_string(), "otel.endpoint");
        assert_eq!(path.keys().collect::<Vec<_>>(), vec!["otel", "endpoint"]);
    }
}
