use lapin::message::Delivery;
use lapin::types::{AMQPValue, ShortString};
use opentelemetry::propagation::{Extractor, Injector};
use std::collections::BTreeMap;
use tokio::time;
use tracing::subscriber::SetGlobalDefaultError;
use tracing::{error, info_span, warn, Span};
use tracing_opentelemetry::OpenTelemetrySpanExt;
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

pub struct AmqpHeaderCarrier<'a> {
    headers: &'a BTreeMap<ShortString, AMQPValue>,
}

impl<'a> AmqpHeaderCarrier<'a> {
    pub(crate) fn new(headers: &'a BTreeMap<ShortString, AMQPValue>) -> Self {
        Self { headers }
    }
}

impl<'a> Extractor for AmqpHeaderCarrier<'a> {
    fn get(&self, key: &str) -> Option<&str> {
        self.headers.get(key).and_then(|header_value| {
            if let AMQPValue::LongString(header_value) = header_value {
                std::str::from_utf8(header_value.as_bytes())
                    .map_err(|e| error!("Error decoding header value {:?}", e))
                    .ok()
            } else {
                warn!("Missing amqp tracing context propagation");
                None
            }
        })
    }

    fn keys(&self) -> Vec<&str> {
        self.headers.keys().map(|header| header.as_str()).collect()
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

pub fn correlate_trace_from_delivery(delivery: Delivery) -> (Span, Delivery) {
    let span = info_span!("correlate_trace_from_delivery");

    let headers = &delivery
        .properties
        .headers()
        .clone()
        .unwrap_or_default()
        .inner()
        .clone();

    let parent_cx = opentelemetry::global::get_text_map_propagator(|propagator| {
        propagator.extract(&AmqpHeaderCarrier::new(headers))
    });

    span.set_parent(parent_cx);

    (span, delivery)
}
