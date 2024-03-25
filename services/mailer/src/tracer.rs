use crate::config::app_config;
use opentelemetry::sdk::trace::BatchConfig;
use tracing_subscriber::{prelude::__tracing_subscriber_SubscriberExt, EnvFilter, Registry};

/// Initializes application tracing, exporting spans to Jaeger
pub fn init() {
    let tracer_service_name = &app_config().tracer_service_name;

    opentelemetry::global::set_text_map_propagator(opentelemetry_jaeger::Propagator::new());

    let tracer = opentelemetry_jaeger::new_agent_pipeline()
        .with_service_name(tracer_service_name)
        .with_auto_split_batch(true)
        .with_batch_processor_config(BatchConfig::default().with_max_export_batch_size(256))
        .install_batch(opentelemetry::runtime::Tokio)
        .expect("failed to initialize tracer");

    let telemetry = tracing_opentelemetry::layer().with_tracer(tracer);

    let subscriber = Registry::default()
        .with(telemetry)
        .with(EnvFilter::from_default_env());

    tracing::subscriber::set_global_default(subscriber).expect("failed set tracing subscriber");

    println!("[TRACER] initialized as service: {}", tracer_service_name);
}
