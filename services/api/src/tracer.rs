use tracing::subscriber::SetGlobalDefaultError;
use tracing_subscriber::{prelude::__tracing_subscriber_SubscriberExt, Registry};

/// initialize the API tracing, creating a layer that exports spans to Jaeger
/// using opentelemetry and a stdout layer if `with_stdout` is true
pub fn init(service_name: &str, with_stdout: bool) -> Result<(), SetGlobalDefaultError> {
    // opentelemetry_jaeger DEPRECATED
    opentelemetry::global::set_text_map_propagator(opentelemetry_jaeger::Propagator::new());

    let tracer = opentelemetry_jaeger::new_agent_pipeline()
        .with_service_name(service_name)
        .with_auto_split_batch(true)
        .install_batch(opentelemetry::runtime::Tokio)
        .expect("failed to initialize tracer");

    let telemetry_layer = tracing_opentelemetry::layer().with_tracer(tracer);

    let stdout_layer = if with_stdout {
        Some(tracing_subscriber::fmt::Layer::default())
    } else {
        None
    };

    let subscriber = Registry::default().with(stdout_layer).with(telemetry_layer);

    tracing::subscriber::set_global_default(subscriber)?;

    println!("[TRACER] initialized");
    Ok(())
}
