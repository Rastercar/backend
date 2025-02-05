use config::AppConfig;
use rabbitmq::{RmqListener, RmqMessage};
use server::{h02, listeners};
use signal_hook::{
    consts::{SIGINT, SIGTERM},
    iterator::Signals,
};
use std::sync::Arc;
use tokio::sync::mpsc;

mod config;
mod http;
mod protocols;
mod rabbitmq;
mod server;

#[tokio::main]
#[allow(clippy::never_loop)]
async fn main() {
    let config = AppConfig::from_env();

    shared::tracer::init_tracing_with_jaeger_otel(shared::tracer::TracingOpts {
        service_name: config.tracer_service_name.clone(),
        exporter_endpoint: config.otel_exporter_otlp_endpoint.clone(),
        with_std_out_layer: config.app_debug,
    });

    let mut signals = Signals::new([SIGINT, SIGTERM]).expect("failed to setup signals hook");

    let (sender, receiver) = mpsc::unbounded_channel::<(RmqMessage, tracing::Span)>();

    let rmq_server = Arc::new(RmqListener::new(&config, receiver));
    let rmq_server_ref = rmq_server.clone();

    tokio::spawn(async move { rmq_server.start().await });
    tokio::spawn(async move { http::start_server(config.http_port).await });

    tokio::spawn(async move {
        for sig in signals.forever() {
            println!("\n[APP] received signal: {}, shutting down", sig);

            shared::tracer::shutdown().await;
            rmq_server_ref.shutdown().await;

            std::process::exit(sig)
        }
    });

    listeners::start_tcp_listener(
        format!("0.0.0.0:{}", config.port_h02).as_str(),
        sender,
        h02::stream_handler,
    )
    .await
    .unwrap();
}
