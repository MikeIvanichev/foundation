#![cfg(feature = "config")]

use std::collections::BTreeMap;

use foundation::config::Renderer;
use foundation_macros::FoundationConfig;
use url::Url;

#[derive(Debug, FoundationConfig, PartialEq)]
struct SimpleConfig {
    /// Service display name.
    #[serde(default = "default_simple_name")]
    name: String,
    /// Port to bind.
    port: u16,
    /// Logging settings.
    logging: SimpleLogging,
    /// Optional labels.
    #[serde(default)]
    labels: BTreeMap<String, String>,
}

#[derive(Debug, FoundationConfig, PartialEq)]
struct SimpleLogging {
    /// Enables JSON logging.
    #[serde(default)]
    json: bool,
    /// Optional extra targets.
    #[serde(default)]
    extra_targets: Vec<String>,
}

fn default_simple_name() -> String {
    "foundation".to_owned()
}

#[allow(dead_code)]
#[derive(Debug, FoundationConfig)]
struct ComplexConfig {
    /// Human-readable service name.
    #[serde(default = "default_complex_name")]
    service_name: String,

    /// Port to bind.
    port: u16,

    /// Logging configuration.
    #[serde(default)]
    logging: LoggingConfig,

    /// Gateway configuration.
    #[serde(default)]
    gateway: GatewayConfig,
}

#[allow(dead_code)]
#[derive(Debug, Default, FoundationConfig)]
struct LoggingConfig {
    /// Enables JSON logs.
    #[serde(default)]
    json: bool,

    /// Minimum log level to emit.
    #[serde(default = "default_log_level")]
    level: String,

    /// Additional log targets to enable.
    #[serde(default)]
    extra_targets: Vec<String>,
}

#[allow(dead_code)]
#[derive(Debug, Default, FoundationConfig)]
struct GatewayConfig {
    /// Public HTTP gateway.
    #[serde(default)]
    http: HttpGatewayConfig,

    /// Listener transport selection.
    #[serde(default)]
    listener: ListenerConfig,
}

#[allow(dead_code)]
#[derive(Debug, Default, FoundationConfig)]
struct HttpGatewayConfig {
    /// Primary upstream endpoint.
    #[serde(default)]
    endpoint: EndpointConfig,
}

#[allow(dead_code)]
#[derive(Debug, FoundationConfig)]
struct EndpointConfig {
    /// Base URL for upstream requests.
    #[serde(default = "default_upstream_url")]
    base_url: Url,
}

impl Default for EndpointConfig {
    fn default() -> Self {
        Self {
            base_url: default_upstream_url(),
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Default, FoundationConfig)]
struct ListenerConfig {
    /// HTTP listener configuration.
    http: Option<ListenerBinding>,

    /// gRPC listener configuration.
    grpc: Option<ListenerBinding>,

    /// TCP listener configuration.
    tcp: Option<ListenerBinding>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Default, FoundationConfig)]
struct ListenerBinding {
    /// Bind address for the listener.
    bind: String,
}

fn default_complex_name() -> String {
    "foundation-example".to_owned()
}

fn default_log_level() -> String {
    "info".to_owned()
}

fn default_upstream_url() -> Url {
    Url::parse("https://api.example.invalid/").expect("default URL")
}

#[test]
fn render_simple_template() {
    let mut out = Vec::new();
    Renderer::new("APP")
        .write_template::<SimpleConfig>(&mut out)
        .expect("render template");
    let out = String::from_utf8(out).expect("rendered UTF-8");
    insta::assert_snapshot!(out);
}

#[test]
fn render_simple_defaults() {
    let mut out = Vec::new();
    Renderer::new("APP")
        .write_defaults::<SimpleConfig>(&mut out)
        .expect("render defaults");
    let out = String::from_utf8(out).expect("rendered UTF-8");
    insta::assert_snapshot!(out);
}

#[test]
fn render_simple_required() {
    let mut out = Vec::new();
    Renderer::new("APP")
        .write_required::<SimpleConfig>(&mut out)
        .expect("render required");
    let out = String::from_utf8(out).expect("rendered UTF-8");
    insta::assert_snapshot!(out);
}

#[test]
fn render_complex_template() {
    let mut out = Vec::new();
    Renderer::new("FOUNDATION")
        .write_template::<ComplexConfig>(&mut out)
        .expect("render template");
    let out = String::from_utf8(out).expect("rendered UTF-8");
    insta::assert_snapshot!(out);
}

#[test]
fn render_complex_defaults() {
    let mut out = Vec::new();
    Renderer::new("FOUNDATION")
        .write_defaults::<ComplexConfig>(&mut out)
        .expect("render defaults");
    let out = String::from_utf8(out).expect("rendered UTF-8");
    insta::assert_snapshot!(out);
}

#[test]
fn render_complex_required() {
    let mut out = Vec::new();
    Renderer::new("FOUNDATION")
        .write_required::<ComplexConfig>(&mut out)
        .expect("render required");
    let out = String::from_utf8(out).expect("rendered UTF-8");
    insta::assert_snapshot!(out);
}

#[test]
fn render_no_prefix_template() {
    let mut out = Vec::new();
    Renderer::new("")
        .write_template::<SimpleConfig>(&mut out)
        .expect("render template");
    let out = String::from_utf8(out).expect("rendered UTF-8");
    insta::assert_snapshot!(out);
}
