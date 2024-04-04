use super::decoder::h02;
use crate::{modules::globals::TRACKER_ID_CACHE, rabbitmq::Rmq};
use lapin::{message::Delivery, options::BasicConsumeOptions, types::FieldTable};
use sea_orm::DatabaseConnection;
use socketioxide::SocketIo;
use std::{sync::Arc, time::Duration};
use tracing::{error, warn, Instrument};

/// handler for tracker events recieved from the decoder microservice through a
/// RabbitMQ delivery, this mainly passes the message to the appropriate function
/// based on the `protocol`, `event_type` and the `imei` on the delivery routing key
#[tracing::instrument(skip_all)]
async fn on_tracker_event(delivery: Delivery, db: &DatabaseConnection, socket: &SocketIo) {
    let routing_key = delivery.routing_key.to_string();

    // tracking events routing keys have the following pattern
    // {protocol}.{type}.{imei}
    //
    // - protocol: the original protocol of the tracker
    // - type: eventy type, eg: "position", "alert", "heartbeat"
    // - imei: the tracking device IMEI
    let [protocol, event_type, imei]: [&str; 3] = routing_key
        .split('.')
        .collect::<Vec<&str>>()
        .try_into()
        .unwrap_or_default();

    if protocol.is_empty() || event_type.is_empty() || imei.is_empty() {
        error!("invalid tracker event routing key: {}", routing_key);
        return;
    }

    // it might seem dumb to rejoin protocol and event_type
    // again but it was needed to separate by '.' in three parts
    // to check if the routing key was valid
    let protocol_and_event = protocol.to_owned() + "." + event_type;

    // for now we only support the h02 protocol and the location message
    // when this grows we should move this to a decoder struct that maps
    // the combination of protocol and event_type to a struct that implements
    // serializable
    if protocol_and_event != "h02.location" {
        error!("unsupported protocol and/or event {protocol_and_event}");
        return;
    }

    let tracker_cache = TRACKER_ID_CACHE
        .get()
        .expect("tracker id cache not initialized");

    let tracker_id: i32 = match tracker_cache.write().await.get(imei).await {
        Some(id) => id,
        None => {
            warn!("tracker: {imei} does not exist");
            return;
        }
    };

    let _ = h02::handle_location(&delivery, socket, tracker_id, db).await;
}

/// Starts a RabbitMQ consumer that listens for any tracker event
/// on the tracker events queue.
///
/// this is supossed to run for the entirety of the program, so
/// it attempts to reconnect infinitely if the connection ends and
/// thus so does the consumer.
pub fn start_positions_consumer(rmq: Arc<Rmq>, socket_io: SocketIo, db: DatabaseConnection) {
    tokio::task::spawn(async move {
        // Important: use automatic acknowledgement mode because we will recieve a
        // lot of positions per seconds and we dont really care if a tiny few are lost
        let consume_options = BasicConsumeOptions {
            no_ack: true,
            ..Default::default()
        };

        let db_ref = &db;
        let socket_ref = &socket_io;

        loop {
            tokio::time::sleep(Duration::from_secs(5)).await;
            println!("[RMQ] starting tracker positions consumer");

            let consume_end_result = rmq
                .consume(
                    shared::constants::rabbitmq::TRACKER_EVENTS_QUEUE,
                    "api_tracker_events_consumer",
                    consume_options,
                    FieldTable::default(),
                    |delivery: Delivery| async move {
                        let (span, delivery) =
                            shared::tracer::correlate_trace_from_delivery(delivery);

                        on_tracker_event(delivery, db_ref, socket_ref)
                            .instrument(span)
                            .await
                    },
                )
                .await;

            if let Err(error) = consume_end_result {
                error!("[RMQ] tracker positions consumer error {error}");
            }
        }
    });
}
