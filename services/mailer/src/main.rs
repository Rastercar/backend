use lapin::message::Delivery;
use mailer::Mailer;
use queue::{controller::router::QueueRouter, MailerRabbitmq};
use signal_hook::{
    consts::{SIGINT, SIGTERM},
    iterator::Signals,
};
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::Instrument;

mod config;
mod http;
mod mailer;
mod queue;
mod tracer;
mod utils;

#[tokio::main]
async fn main() {
    tracer::init();

    let (sender, mut receiver) = mpsc::unbounded_channel::<Delivery>();

    let mailer_rmq = Arc::new(MailerRabbitmq::new(sender));

    let mailer = Mailer::new(mailer_rmq.clone()).await;
    let router = Arc::new(QueueRouter::new(mailer_rmq.clone(), mailer));

    let mailer_rmq_ref = mailer_rmq.clone();
    let shutdown_mailer_rmq_ref = mailer_rmq.clone();

    tokio::spawn(async move { mailer_rmq.clone().start_consumer().await });
    tokio::spawn(async move { http::server::start(mailer_rmq_ref).await });

    listen_to_shutdown_signals(shutdown_mailer_rmq_ref);

    while let Some(delivery) = receiver.recv().await {
        let (span, delivery) = shared::tracer::correlate_trace_from_delivery(delivery);
        let router = router.clone();
        tokio::spawn(async move { router.handle_delivery(delivery).instrument(span).await });
    }
}

/// Listen to shutdown signals `SIGINT` and `SIGTERM`, on a signal gracefully shutdowns down the application
#[allow(clippy::never_loop)]
fn listen_to_shutdown_signals(rmq: Arc<MailerRabbitmq>) {
    let mut signals = Signals::new([SIGINT, SIGTERM]).expect("failed to setup signals hook");

    tokio::spawn(async move {
        for sig in signals.forever() {
            println!("\n[APP] received signal: {}, shutting down", sig);

            shared::tracer::shutdown().await;
            rmq.shutdown().await;

            std::process::exit(sig)
        }
    });
}
