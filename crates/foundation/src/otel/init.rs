use std::time::Duration;

use http::HeaderMap;
use http::HeaderName;
use http::HeaderValue;
use opentelemetry::KeyValue;
use opentelemetry::global;
use opentelemetry::trace::TracerProvider;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_otlp::WithTonicConfig;
use opentelemetry_otlp::tonic_types::metadata::MetadataMap;
use opentelemetry_otlp::tonic_types::transport::ClientTlsConfig;
use opentelemetry_sdk::Resource;
use opentelemetry_sdk::error::OTelSdkError;
use opentelemetry_sdk::metrics::MeterProviderBuilder;
use opentelemetry_sdk::metrics::PeriodicReader;
use opentelemetry_sdk::metrics::SdkMeterProvider;
use opentelemetry_sdk::propagation::TraceContextPropagator;
use opentelemetry_sdk::trace::RandomIdGenerator;
use opentelemetry_sdk::trace::Sampler;
use opentelemetry_sdk::trace::SdkTracerProvider;
use opentelemetry_semantic_conventions::SCHEMA_URL;
use opentelemetry_semantic_conventions::resource::DEPLOYMENT_ENVIRONMENT_NAME;
use opentelemetry_semantic_conventions::resource::SERVICE_VERSION;
use serde::Deserialize;
use serde::Serialize;
use tracing_opentelemetry::MetricsLayer;
use tracing_opentelemetry::OpenTelemetryLayer;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

/// OpenTelemetry initialization configuration.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Config {
    /// Resource service name.
    pub service_name: String,
    /// Resource service version.
    pub service_version: String,
    /// Resource deployment environment name.
    pub deployment_env_name: String,
    /// OTLP endpoint URL.
    pub endpoint_url: String,
    /// Whether to enable TLS for OTLP connections.
    pub tls_enabled: bool,
    /// Optional gRPC metadata encoded as `key=value,key2=value2`.
    #[serde(default)]
    pub metadata: Metadata,
    /// Period between metric exports.
    #[serde(with = "humantime_serde")]
    pub metrics_export_interval: Duration,
}

impl Config {
    fn build_resource(&self) -> Resource {
        Resource::builder()
            .with_service_name(self.service_name.clone())
            .with_schema_url(
                [
                    KeyValue::new(SERVICE_VERSION, self.service_version.clone()),
                    KeyValue::new(
                        DEPLOYMENT_ENVIRONMENT_NAME,
                        self.deployment_env_name.clone(),
                    ),
                ],
                SCHEMA_URL,
            )
            .build()
    }
}

#[cfg(feature = "config")]
impl foundation_types::config::ConfigSchema for Config {
    fn schema() -> foundation_types::config::Schema {
        use foundation_types::config::Field;
        use foundation_types::config::FieldKind;
        use foundation_types::config::Schema;

        let mut fields = Schema::builder();
        fields.push(Field {
            key: "service_name",
            docs: &["Resource service name."],
            required: true,
            kind: FieldKind::Leaf { default_yaml: None },
        });
        fields.push(Field {
            key: "service_version",
            docs: &["Resource service version."],
            required: true,
            kind: FieldKind::Leaf { default_yaml: None },
        });
        fields.push(Field {
            key: "deployment_env_name",
            docs: &["Resource deployment environment name."],
            required: true,
            kind: FieldKind::Leaf { default_yaml: None },
        });
        fields.push(Field {
            key: "endpoint_url",
            docs: &["OTLP endpoint URL."],
            required: true,
            kind: FieldKind::Leaf { default_yaml: None },
        });
        fields.push(Field {
            key: "tls_enabled",
            docs: &["Whether to enable TLS for OTLP connections."],
            required: true,
            kind: FieldKind::Leaf { default_yaml: None },
        });
        fields.push(Field {
            key: "metadata",
            docs: &["Optional gRPC metadata encoded as `key=value,key2=value2`."],
            required: false,
            kind: FieldKind::Leaf {
                default_yaml: Some("''".to_owned()),
            },
        });
        fields.push(Field {
            key: "metrics_export_interval",
            docs: &["Period between metric exports."],
            required: true,
            kind: FieldKind::Leaf { default_yaml: None },
        });
        fields.build()
    }
}

/// OTLP metadata encoded as a comma-separated `key=value` string.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct Metadata(String);

impl Metadata {
    fn as_metadata_map(&self) -> anyhow::Result<MetadataMap> {
        let mut headers = HeaderMap::new();

        for pair in self
            .0
            .split(',')
            .map(str::trim)
            .filter(|pair| !pair.is_empty())
        {
            let (name, value) = pair
                .split_once('=')
                .ok_or_else(|| anyhow::anyhow!("invalid OTLP metadata pair `{pair}`"))?;

            headers.insert(
                HeaderName::try_from(name.trim())?,
                HeaderValue::from_str(value.trim())?,
            );
        }

        Ok(MetadataMap::from_headers(headers))
    }
}

impl From<String> for Metadata {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for Metadata {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}

/// Guard that keeps OpenTelemetry providers alive until shutdown.
#[must_use = "drop this guard only when telemetry should stop"]
pub struct OtelGuard {
    tracer_provider: SdkTracerProvider,
    meter_provider: SdkMeterProvider,
}

impl OtelGuard {
    /// Flush spans and metrics immediately.
    pub fn force_flush(&self) -> Result<(), OTelSdkError> {
        self.tracer_provider.force_flush()?;
        self.meter_provider.force_flush()?;

        Ok(())
    }
}

impl Drop for OtelGuard {
    fn drop(&mut self) {
        if let Err(error) = self.tracer_provider.shutdown() {
            tracing::error!(%error, "failed to shut down OTEL tracer provider");
        }

        if let Err(error) = self.meter_provider.shutdown() {
            tracing::error!(%error, "failed to shut down OTEL meter provider");
        }
    }
}

/// Initialize tracing, metrics, and trace propagation via OTLP/gRPC.
pub fn init_tracing(config: Config) -> anyhow::Result<OtelGuard> {
    let tracer_provider = init_tracer_provider(&config)?;
    let meter_provider = init_meter_provider(&config)?;

    let tracer = tracer_provider.tracer("foundation::otel");
    let env_filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new("info"))
        .map_err(anyhow::Error::from)?;

    tracing_subscriber::registry()
        .with(env_filter)
        .with(tracing_subscriber::fmt::layer())
        .with(OpenTelemetryLayer::new(tracer))
        .with(MetricsLayer::new(meter_provider.clone()))
        .try_init()
        .map_err(anyhow::Error::from)?;

    global::set_text_map_propagator(TraceContextPropagator::new());
    global::set_tracer_provider(tracer_provider.clone());
    global::set_meter_provider(meter_provider.clone());

    let previous_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        tracing_panic::panic_hook(panic_info);
        previous_hook(panic_info);
    }));

    Ok(OtelGuard {
        tracer_provider,
        meter_provider,
    })
}

fn init_tracer_provider(config: &Config) -> anyhow::Result<SdkTracerProvider> {
    let mut builder = opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .with_endpoint(&config.endpoint_url)
        .with_protocol(opentelemetry_otlp::Protocol::Grpc);

    if config.tls_enabled {
        builder = builder.with_tls_config(ClientTlsConfig::new().with_webpki_roots());
    }

    let exporter = builder
        .with_metadata(config.metadata.as_metadata_map()?)
        .build()
        .map_err(anyhow::Error::from)?;

    Ok(SdkTracerProvider::builder()
        .with_sampler(Sampler::ParentBased(Box::new(Sampler::AlwaysOn)))
        .with_id_generator(RandomIdGenerator::default())
        .with_resource(config.build_resource())
        .with_batch_exporter(exporter)
        .build())
}

fn init_meter_provider(config: &Config) -> anyhow::Result<SdkMeterProvider> {
    let mut builder = opentelemetry_otlp::MetricExporter::builder()
        .with_tonic()
        .with_endpoint(&config.endpoint_url)
        .with_protocol(opentelemetry_otlp::Protocol::Grpc);

    if config.tls_enabled {
        builder = builder.with_tls_config(ClientTlsConfig::new().with_webpki_roots());
    }

    let exporter = builder
        .with_metadata(config.metadata.as_metadata_map()?)
        .with_temporality(opentelemetry_sdk::metrics::Temporality::default())
        .build()
        .map_err(anyhow::Error::from)?;

    let reader = PeriodicReader::builder(exporter)
        .with_interval(config.metrics_export_interval)
        .build();

    Ok(MeterProviderBuilder::default()
        .with_resource(config.build_resource())
        .with_reader(reader)
        .build())
}

#[cfg(test)]
mod tests {
    use super::Metadata;

    #[test]
    fn metadata_parses_empty_string() {
        assert!(Metadata::default().as_metadata_map().is_ok());
    }

    #[test]
    fn metadata_rejects_malformed_pairs() {
        let error = Metadata::from("missing_separator")
            .as_metadata_map()
            .expect_err("expected malformed metadata to fail");

        assert!(error.to_string().contains("invalid OTLP metadata pair"));
    }
}
