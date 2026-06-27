use foundation::config::Renderer;
use foundation_macros::FoundationConfig;

#[allow(dead_code)]
#[derive(FoundationConfig)]
#[serde(rename_all = "kebab-case")]
struct ServiceConfig {
    /// Service display name.
    #[serde(default = "default_service_name")]
    service_name: String,

    /// Optional listener.
    listener: Option<ListenerConfig>,
}

#[allow(dead_code)]
#[derive(FoundationConfig)]
struct ListenerConfig {
    /// Bind address.
    bind: String,
}

fn default_service_name() -> String {
    "integration".to_owned()
}

#[test]
fn renders_config_template_across_crates() {
    let mut out = Vec::new();
    Renderer::new("APP")
        .write_template::<ServiceConfig>(&mut out)
        .expect("render template");
    let out = String::from_utf8(out).expect("rendered UTF-8");

    insta::assert_snapshot!(out);
}

#[test]
fn renders_foundation_otel_config_schema() {
    let mut out = Vec::new();
    Renderer::new("APP")
        .write_required::<foundation::otel::Config>(&mut out)
        .expect("render required config");
    let out = String::from_utf8(out).expect("rendered UTF-8");

    insta::assert_snapshot!(out);
}
