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
mod errors;
mod protocols;
mod rabbitmq;
mod server;
mod tracer;

#[tokio::main]
#[allow(clippy::never_loop)]
async fn main() {
    let config = AppConfig::from_env().expect("failed to load application config");

    tracer::init(config.tracer_service_name.to_owned()).expect("failed to init tracer");

    let mut signals = Signals::new([SIGINT, SIGTERM]).expect("failed to setup signals hook");

    let (sender, receiver) = mpsc::unbounded_channel::<(RmqMessage, tracing::Span)>();

    let rmq_server = Arc::new(RmqListener::new(&config, receiver));
    let rmq_server_ref = rmq_server.clone();

    tokio::spawn(async move { rmq_server.start().await });

    tokio::spawn(async move {
        for sig in signals.forever() {
            println!("\n[APP] received signal: {}, shutting down", sig);

            shared::tracer::shutdown().await;
            rmq_server_ref.shutdown().await;

            std::process::exit(sig)
        }
    });

    listeners::start_tcp_listener(
        format!("127.0.0.1:{}", config.port_h02).as_str(),
        sender,
        h02::stream_handler,
    )
    .await
    .unwrap();
}
