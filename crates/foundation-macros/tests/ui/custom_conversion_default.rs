use foundation_macros::FoundationConfig;

#[allow(dead_code)]
#[derive(FoundationConfig)]
struct Config {
    #[serde(default, deserialize_with = "deserialize_value")]
    value: String,
}

fn main() {}
