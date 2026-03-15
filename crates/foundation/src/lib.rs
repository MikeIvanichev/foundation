#![cfg_attr(docsrs, feature(doc_cfg))]

//! For Empire, the foundation we lay.
//!
//! Shared service foundations for the Skuld Systems Rust workspace.

#[cfg(feature = "config")]
#[cfg_attr(docsrs, doc(cfg(feature = "config")))]
pub mod config;
#[cfg(feature = "jemalloc")]
mod jemalloc;
#[cfg(feature = "otel")]
#[cfg_attr(docsrs, doc(cfg(feature = "otel")))]
pub mod otel;

#[cfg(feature = "jemalloc")]
pub use jemalloc::DEFAULT_MALLOC_CONF;
#[cfg(feature = "jemalloc")]
pub use jemalloc::MALLOC_CONF;

#[cfg(feature = "config")]
#[cfg_attr(docsrs, doc(cfg(feature = "config")))]
pub use foundation_types as config_types;

/// Basic service metadata.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ServiceInfo {
    /// Human-readable service name.
    pub name: &'static str,
    /// Service identifier as used in metrics.
    pub name_in_metrics: String,
    /// Service version.
    pub version: &'static str,
    /// Service author.
    pub author: &'static str,
    /// Service description.
    pub description: &'static str,
    /// Canonical env prefix used for configuration overlays.
    pub config_env_prefix: String,
}

impl ServiceInfo {
    /// Returns the canonical env prefix for the service.
    #[must_use]
    pub fn conventional_config_env_prefix(name: &str) -> String {
        let mut prefix = String::with_capacity(name.len());

        for ch in name.chars() {
            match ch {
                '-' | '_' => prefix.push_str("__"),
                ch => prefix.push(ch.to_ascii_uppercase()),
            }
        }

        prefix
    }
}

/// Creates [`ServiceInfo`] from the active package manifest.
#[macro_export]
macro_rules! service_info {
    () => {
        $crate::ServiceInfo {
            name: env!("CARGO_PKG_NAME"),
            name_in_metrics: env!("CARGO_PKG_NAME").replace('-', "_"),
            version: env!("CARGO_PKG_VERSION"),
            author: env!("CARGO_PKG_AUTHORS"),
            description: env!("CARGO_PKG_DESCRIPTION"),
            config_env_prefix: $crate::ServiceInfo::conventional_config_env_prefix(env!(
                "CARGO_PKG_NAME"
            )),
        }
    };
}
