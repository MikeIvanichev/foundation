use foundation_macros::FoundationConfig;

#[allow(dead_code)]
#[derive(FoundationConfig)]
struct Config {
    #[serde(default = "default_nested")]
    nested: NestedConfig,
}

#[allow(dead_code)]
#[derive(Default, FoundationConfig)]
struct NestedConfig {
    value: String,
}

fn default_nested() -> NestedConfig {
    NestedConfig::default()
}

fn main() {}
