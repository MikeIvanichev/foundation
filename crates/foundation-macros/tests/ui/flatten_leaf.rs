use foundation_macros::FoundationConfig;

#[allow(dead_code)]
#[derive(FoundationConfig)]
struct Config {
    #[serde(flatten)]
    value: String,
}

fn main() {}
