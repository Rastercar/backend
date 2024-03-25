use lapin::{
    message::Delivery,
    types::{AMQPValue, ShortString},
};
use opentelemetry::{
    propagation::{Extractor, Injector},
    Context,
};
use std::collections::BTreeMap;
use tokio::time;
use tracing::{error, info_span, warn, Span};
use tracing_opentelemetry::OpenTelemetrySpanExt;

/// struct to Injecting and Extracting otel span contexts into/from a
/// rabbitmq delivery using its headers
pub struct AmqpHeaderCarrier<'a> {
    headers: &'a mut BTreeMap<ShortString, AMQPValue>,
}

impl<'a> AmqpHeaderCarrier<'a> {
    pub fn new(headers: &'a mut BTreeMap<ShortString, AMQPValue>) -> Self {
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

impl<'a> Injector for AmqpHeaderCarrier<'a> {
    fn set(&mut self, key: &str, value: String) {
        self.headers
            .insert(key.into(), AMQPValue::LongString(value.into()));
    }
}

/// create a BTreeMap containing the injected context of a span
pub fn create_amqp_headers_with_span_ctx(ctx: &Context) -> BTreeMap<ShortString, AMQPValue> {
    let mut amqp_headers = BTreeMap::new();

    // inject the current context through the amqp headers
    opentelemetry::global::get_text_map_propagator(|propagator| {
        propagator.inject_context(ctx, &mut AmqpHeaderCarrier::new(&mut amqp_headers))
    });

    amqp_headers
}

/// Extracts the text map propagator from the AMQP headers and creates a span
/// with the extracted context as the parent context.
pub fn correlate_trace_from_delivery(delivery: Delivery) -> (Span, Delivery) {
    let span = info_span!("correlate_trace_from_delivery");

    let headers = &mut delivery
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

/// async wrapper for `opentelemetry::global::shutdown_tracer_provider()` because it might hang forever
///
///  see: https://github.com/open-telemetry/opentelemetry-rust/issues/868
async fn shutdown_trace_provider() {
    println!("[TRACER] shutting down");
    opentelemetry::global::shutdown_tracer_provider();
}

/// Shutdowns tracing with a 500 millisecond timeout to export all non exported spans.
pub async fn shutdown() {
    tokio::select! {
        _ = time::sleep(time::Duration::from_millis(500)) => {
            eprintln!("[TRACER] gracefull shutdown failed");
        },
        _ = tokio::task::spawn_blocking(shutdown_trace_provider) => {
            println!("[TRACER] gracefull shutdown ok");
        }
    }
}
