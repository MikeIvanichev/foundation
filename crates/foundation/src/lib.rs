#![cfg_attr(docsrs, feature(doc_cfg))]

//! For Empire, the foundation we lay.
//!
//! Shared service foundations for the Skuld Systems Rust workspace.

#[cfg(feature = "jemalloc")]
mod alloc;
#[cfg(feature = "config")]
#[cfg_attr(docsrs, doc(cfg(feature = "config")))]
pub mod config;
#[cfg(feature = "otel")]
#[cfg_attr(docsrs, doc(cfg(feature = "otel")))]
pub mod otel;

#[cfg(feature = "jemalloc")]
pub use alloc::DEFAULT_MALLOC_CONF;
#[cfg(feature = "jemalloc")]
pub use alloc::MALLOC_CONF;

#[cfg(feature = "config")]
#[cfg_attr(docsrs, doc(cfg(feature = "config")))]
pub use foundation_types;

/// Metadata that identifies a service across foundation systems.
///
/// This is usually created with [`service_info!`], but can be constructed
/// directly when a service needs custom identifiers.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ServiceInfo {
    /// The name of the service.
    pub name: &'static str,
    /// The service identifier as used in metrics.
    ///
    /// Usually the same as [`ServiceInfo::name`], with hyphens (`-`) replaced
    /// by underscores (`_`).
    pub name_in_metrics: String,
    /// The environment variable prefix used for service configuration.
    ///
    /// Usually derived from [`ServiceInfo::name`] by converting ASCII letters
    /// to uppercase and replacing hyphens (`-`) and underscores (`_`) with
    /// double underscores (`__`).
    pub env_prefix: String,
    /// The version of the service.
    pub version: &'static str,
    /// The service author.
    pub author: &'static str,
    /// The description of the service.
    pub description: &'static str,
}

/// Creates [`ServiceInfo`] from the `Cargo.toml` manifest of the calling
/// service.
///
/// [`ServiceInfo::name`] is set to the package name.
/// [`ServiceInfo::name_in_metrics`] is the package name with hyphens (`-`)
/// replaced by underscores (`_`).
/// [`ServiceInfo::env_prefix`] is the package name converted to ASCII
/// uppercase, with hyphens (`-`) and underscores (`_`) replaced by double
/// underscores (`__`).
#[macro_export]
macro_rules! service_info {
    () => {
        $crate::ServiceInfo {
            name: env!("CARGO_PKG_NAME"),
            name_in_metrics: env!("CARGO_PKG_NAME").replace('-', "_"),
            env_prefix: {
                let name = env!("CARGO_PKG_NAME");
                let mut prefix = String::with_capacity(name.len());

                for ch in name.chars() {
                    match ch {
                        '-' | '_' => prefix.push_str("__"),
                        ch => prefix.push(ch.to_ascii_uppercase()),
                    }
                }

                prefix
            },
            version: env!("CARGO_PKG_VERSION"),
            author: env!("CARGO_PKG_AUTHORS"),
            description: env!("CARGO_PKG_DESCRIPTION"),
        }
    };
}
