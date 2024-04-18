use tracing::subscriber::SetGlobalDefaultError;
use tracing_subscriber::{prelude::__tracing_subscriber_SubscriberExt, Registry};

pub fn init(service_name: &str) -> Result<(), SetGlobalDefaultError> {
    opentelemetry::global::set_text_map_propagator(opentelemetry_jaeger::Propagator::new());

    let tracer = opentelemetry_jaeger::new_agent_pipeline()
        .with_service_name(service_name)
        .with_auto_split_batch(true)
        .install_batch(opentelemetry::runtime::Tokio)
        .expect("failed to initialize tracer");

    let telemetry = tracing_opentelemetry::layer().with_tracer(tracer);
    let subscriber = Registry::default().with(telemetry);

    tracing::subscriber::set_global_default(subscriber)?;

    // For quick debuging by printing to stdout uncomment bellow
    // tracing::subscriber::set_global_default(tracing_subscriber::FmtSubscriber::new()).unwrap()

    println!("[TRACER] initialized");
    Ok(())
}
