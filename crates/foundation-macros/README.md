# `foundation-macros`

`FoundationConfig` derives documentation metadata for config structs.

## Dependency requirement

If a derived field default needs to be rendered as YAML, the generated code calls
`serde_saphyr::to_string(...)` directly. That means crates using
`#[derive(FoundationConfig)]` must also depend on `serde-saphyr`.

Typical dependency:

```toml
[dependencies]
foundation-macros = { path = "../foundation-macros" }
foundation-types = { path = "../foundation-types" }
serde-saphyr = { version = "0.0.23", default-features = false, features = ["serialize"] }
```

This is intentional. The derive output references `serde-saphyr` directly so the
consumer controls the exact version in its own dependency graph.
