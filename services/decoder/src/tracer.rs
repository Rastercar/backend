use lapin::types::{AMQPValue, ShortString};
use opentelemetry::propagation::Injector;
use std::collections::BTreeMap;
use tokio::time;
use tracing::subscriber::SetGlobalDefaultError;
use tracing_subscriber::{prelude::__tracing_subscriber_SubscriberExt, Registry};

pub fn init(service_name: String) -> Result<(), SetGlobalDefaultError> {
    opentelemetry::global::set_text_map_propagator(opentelemetry_jaeger::Propagator::new());

    let tracer = opentelemetry_jaeger::new_agent_pipeline()
        .with_service_name(service_name)
        .with_auto_split_batch(true)
        .install_batch(opentelemetry::runtime::Tokio)
        .expect("failed to initialize tracer");

    let telemetry = tracing_opentelemetry::layer().with_tracer(tracer);
    let subscriber = Registry::default().with(telemetry);

    tracing::subscriber::set_global_default(subscriber)?;

    println!("[TRACER] initialized");
    Ok(())
}

// async wrapper for `opentelemetry::global::shutdown_tracer_provider()` because it might hang forever
// see: https://github.com/open-telemetry/opentelemetry-rust/issues/868
async fn shutdown_trace_provider() {
    opentelemetry::global::shutdown_tracer_provider();
}

pub async fn shutdown() {
    println!("[TRACER] shutting down");

    tokio::select! {
        _ = time::sleep(time::Duration::from_millis(500)) => {
            println!("[TRACER] gracefull shutdown failed");
        },
        _ = tokio::task::spawn_blocking(shutdown_trace_provider) => {
            println!("[TRACER] gracefull shutdown ok");
        }
    }
}

pub struct AmqpClientCarrier<'a> {
    properties: &'a mut BTreeMap<ShortString, AMQPValue>,
}

impl<'a> AmqpClientCarrier<'a> {
    pub fn new(properties: &'a mut BTreeMap<ShortString, AMQPValue>) -> Self {
        Self { properties }
    }
}

impl<'a> Injector for AmqpClientCarrier<'a> {
    fn set(&mut self, key: &str, value: String) {
        self.properties
            .insert(key.into(), AMQPValue::LongString(value.into()));
    }
}
