use config::{Config, Environment, File};
use serde::Deserialize;
use std::env;

#[derive(Deserialize, Debug)]
pub struct AppConfig {
    /// If the application should be run in debug mode and print additional info to stdout
    pub app_debug: bool,

    /// Rabbitmq uri
    pub rmq_uri: String,

    /// Name of the exchange to publish email events (positions, alarms, etc)
    pub tracker_events_exchange: String,

    /// The service name to be used on the tracing spans
    pub tracer_service_name: String,

    /// Default port to listen for trackers with the H02 protocol
    pub port_h02: usize,

    /// opentelemetry exporter endpoint
    pub otel_exporter_otlp_endpoint: String,

    /// port to open a HTTP server for service healthchecks
    pub http_port: u16,
}

impl AppConfig {
    pub fn from_env() -> AppConfig {
        let run_mode = env::var("RUN_MODE").unwrap_or_else(|_| "development".into());
        let base_path = env::var("CARGO_MANIFEST_DIR").unwrap_or_default();

        let yaml_config_file = File::with_name(&format!("{base_path}/env/{run_mode}.yaml"))
            .format(config::FileFormat::Yaml)
            .required(false);

        Config::builder()
            .add_source(yaml_config_file)
            .add_source(Environment::default())
            .build()
            .unwrap_or_else(|error| panic!("[CFG] error loading config, {:#?}", error))
            .try_deserialize::<AppConfig>()
            .unwrap_or_else(|error| panic!("[CFG] error deserializing config, {:#?}", error))
    }
}
