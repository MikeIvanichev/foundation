use std::io;
use std::io::Write as _;
use std::process::ExitCode;

use foundation::config::Renderer;
use foundation::service_info;
use foundation_macros::FoundationConfig;
use url::Url;

#[allow(dead_code)]
#[derive(Debug, FoundationConfig)]
struct ExampleConfig {
    /// Human-readable service name.
    #[serde(default = "default_service_name")]
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
#[derive(Debug, FoundationConfig)]
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

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            json: false,
            level: default_log_level(),
            extra_targets: Vec::new(),
        }
    }
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

fn default_service_name() -> String {
    "foundation-example".to_owned()
}

fn default_log_level() -> String {
    "info".to_owned()
}

fn default_upstream_url() -> Url {
    Url::parse("https://api.example.invalid/").expect("default upstream URL must be valid")
}

fn main() -> ExitCode {
    match try_main() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            let stderr = io::stderr();
            let mut stderr = stderr.lock();
            let _ = writeln!(stderr, "Error: {error}");
            ExitCode::FAILURE
        }
    }
}

fn try_main() -> Result<(), String> {
    let service_info = service_info!();
    let mut args = std::env::args_os().skip(1).collect::<Vec<_>>();
    let stdout = io::stdout();
    let mut stdout = stdout.lock();

    if args.is_empty() {
        return Err("missing config subcommand".to_owned());
    }

    if args.remove(0) != "config" {
        return Err("unsupported example CLI command".to_owned());
    }

    let Some(command) = args.first().and_then(|arg| arg.to_str()) else {
        return Err("missing config subcommand".to_owned());
    };

    match command {
        "template" => {
            Renderer::for_service(&service_info).write_template::<ExampleConfig>(&mut stdout)
        }
        "defaults" => {
            Renderer::for_service(&service_info).write_defaults::<ExampleConfig>(&mut stdout)
        }
        "required" => {
            Renderer::for_service(&service_info).write_required::<ExampleConfig>(&mut stdout)
        }
        _ => return Err(format!("unsupported config subcommand `{command}`")),
    }
    .map_err(|error| error.to_string())
}
