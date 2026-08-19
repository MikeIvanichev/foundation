use std::io;
use std::io::Write as _;
use std::net::SocketAddr;
use std::path::Path;
use std::path::PathBuf;

use anyhow::Context as _;
use foundation_macros::FoundationConfig;
use serde::Deserialize;

#[derive(Debug, Deserialize, FoundationConfig)]
struct Config {
    /// Address on which the service listens.
    bind: SocketAddr,

    /// OpenTelemetry export configuration.
    otel: foundation::otel::Config,
}

fn main() -> anyhow::Result<()> {
    let path = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(default_path);
    let config = read_config(&path)?;

    writeln!(
        io::stdout().lock(),
        "loaded {}: service={} bind={}",
        path.display(),
        config.otel.service_name,
        config.bind,
    )?;

    Ok(())
}

fn read_config(path: &Path) -> anyhow::Result<Config> {
    let source = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read config `{}`", path.display()))?;

    serde_saphyr::from_str(&source)
        .with_context(|| format!("failed to parse config `{}`", path.display()))
}

fn default_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/config/read-config.yaml")
}
