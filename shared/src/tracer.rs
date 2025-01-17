use lapin::{
    message::Delivery,
    types::{AMQPValue, ShortString},
};
use opentelemetry::trace::TracerProvider as _;
use opentelemetry::{
    propagation::{Extractor, Injector},
    Context, KeyValue,
};
use opentelemetry_sdk::{runtime, trace::TracerProvider, Resource};
use opentelemetry_semantic_conventions::resource::SERVICE_NAME;
use std::collections::BTreeMap;
use tokio::time;
use tracing::{error, info_span, warn, Span};
use tracing_opentelemetry::OpenTelemetrySpanExt;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

/// struct to Injecting and Extracting OTEL span contexts into/from a
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

/// # PANICS
///
/// when failing to initialize tracing or set globals
///
/// # TRACING INIT
///
/// This should be a part of your application bootstrap code, before any code
/// that uses the tracing crate is called
///
/// Starts the tracing module with a open telemetry layer that will export the spans using
/// the jaeger text map propagator to a jaeger GRPC endpoint, keep in mind that traces are filtered
/// using tracing_subscriber::EnvFilter
///
/// If any of the following is not true **JAEGER TRACING WONT WORK**:
///
/// - your code is running on the TOKIO runtime, otherwise it will break
/// - you are using jaeger 2.0 with a open GRPC port, default port for GRPC is 4317
///
/// this will set the following globals:
///
/// - opentelemetry::global::set_text_map_propagator
/// - opentelemetry::global::set_tracer_provider
/// - global tracing subscriber (https://docs.rs/tracing/0.1.21/tracing/dispatcher/index.html#setting-the-default-subscriber)
///
pub fn init_tracing_with_jaeger_otel(service_name: String, with_std_out_layer: bool) {
    let text_map_propagator = opentelemetry_jaeger_propagator::propagator::Propagator::new();
    opentelemetry::global::set_text_map_propagator(text_map_propagator);

    let exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .build()
        .expect("failed to initialize tracer");

    let provider = TracerProvider::builder()
        .with_batch_exporter(exporter, runtime::Tokio)
        .with_resource(Resource::new(vec![KeyValue::new(
            SERVICE_NAME,
            String::from(service_name.clone()),
        )]))
        .build();

    opentelemetry::global::set_tracer_provider(provider.clone());

    let otel_tracer = provider.tracer(service_name.clone());

    let otel_layer = tracing_opentelemetry::layer().with_tracer(otel_tracer);

    let stdout_layer = if with_std_out_layer {
        Some(tracing_subscriber::fmt::Layer::default())
    } else {
        None
    };

    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .with(stdout_layer)
        .with(otel_layer)
        .init();

    println!("[TRACER] initialized as service: {}", service_name);
}

/// async wrapper for `opentelemetry::global::shutdown_tracer_provider()` because it might hang forever
///
///  see: https://github.com/open-telemetry/opentelemetry-rust/issues/868
async fn shutdown_trace_provider() {
    println!("[TRACER] shutting down");
    opentelemetry::global::shutdown_tracer_provider();
}

/// # TRACING SHUTDOWN
///
/// Shutdowns tracing with a 5 second timeout to export all non exported spans.
///
/// basically a wrapper for opentelemetry::global::shutdown_tracer_provider()
pub async fn shutdown() {
    tokio::select! {
        _ = time::sleep(time::Duration::from_secs(5)) => {
            eprintln!("[TRACER] gracefull shutdown failed");
        },
        _ = tokio::task::spawn_blocking(shutdown_trace_provider) => {
            println!("[TRACER] gracefull shutdown ok");
        }
    }
}
