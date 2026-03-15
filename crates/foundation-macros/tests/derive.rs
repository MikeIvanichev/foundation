use foundation_macros::FoundationConfig;
use foundation_types::config::ConfigSchema;
use foundation_types::config::FieldKind;

fn default_service_name() -> String {
    "foundation".to_owned()
}

fn default_pool_size() -> u16 {
    16
}

#[allow(dead_code)]
#[derive(Debug, FoundationConfig)]
#[serde(rename_all = "kebab-case")]
struct DatabaseConfig {
    /// Maximum connection pool size.
    #[serde(default = "default_pool_size")]
    pool_size: u16,

    /// Replica hosts.
    #[serde(default)]
    replicas: Vec<String>,
}

#[allow(dead_code)]
#[derive(Debug, FoundationConfig)]
#[serde(rename_all = "kebab-case")]
struct ServiceConfig {
    /// Human-readable service name.
    #[serde(default = "default_service_name")]
    service_name: String,

    /// Whether metrics export is enabled.
    #[serde(default)]
    metrics_enabled: bool,

    /// Database settings exposed at the top level.
    #[serde(flatten)]
    database: DatabaseConfig,

    /// Optional per-tenant database overrides.
    tenant_database: Option<DatabaseConfig>,

    #[serde(skip)]
    _skipped: String,
}

#[allow(dead_code)]
#[derive(Debug, FoundationConfig)]
#[serde(rename_all(serialize = "snake_case", deserialize = "kebab-case"))]
struct DeserializeNamingConfig {
    #[serde(rename(serialize = "service_name", deserialize = "service-name"))]
    service_name: String,

    #[serde(skip_deserializing)]
    skipped_input_only: String,
}

#[test]
fn derive_emits_schema_metadata_for_named_structs() {
    let schema = ServiceConfig::schema();

    assert_eq!(schema.fields.len(), 5);

    let service_name = &schema.fields[0];
    assert_eq!(service_name.key, "service-name");
    assert_eq!(service_name.docs, &["Human-readable service name."]);
    assert_eq!(service_name.default_yaml(), Some("foundation"));
    assert!(matches!(service_name.kind, FieldKind::Leaf { .. }));
    assert!(!service_name.required);

    let pool_size = &schema.fields[2];
    assert_eq!(pool_size.key, "pool-size");
    assert_eq!(pool_size.default_yaml(), Some("16"));
    assert!(!pool_size.required);

    let tenant_database = &schema.fields[4];
    assert!(matches!(tenant_database.kind, FieldKind::Nested { .. }));
    assert!(tenant_database.default_yaml().is_none());
    assert!(!tenant_database.required);
}

#[test]
fn derive_records_leaf_defaults_as_yaml_fragments() {
    let schema = ServiceConfig::schema();

    assert_eq!(schema.fields[0].default_yaml(), Some("foundation"));
    assert_eq!(schema.fields[1].default_yaml(), Some("false"));
    assert_eq!(schema.fields[2].default_yaml(), Some("16"));
    assert_eq!(schema.fields[3].default_yaml(), Some("[]"));
    assert!(schema.fields[4].default_yaml().is_none());
}

#[test]
fn derive_uses_deserialize_side_of_serde_names() {
    let schema = DeserializeNamingConfig::schema();

    assert_eq!(schema.fields.len(), 1);
    assert_eq!(schema.fields[0].key, "service-name");
}
