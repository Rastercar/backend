use super::{cache::TrackerIdCache, decoder::h02};
use crate::{
    modules::tracking::dto::PositionDto,
    rabbitmq::{Rmq, TRACKER_EVENTS_QUEUE},
};
use chrono::{DateTime, Utc};
use geozero::wkb;
use lapin::{message::Delivery, options::BasicConsumeOptions, types::FieldTable};
use sea_orm::DatabaseConnection;
use socketioxide::SocketIo;
use sqlx::postgres::PgQueryResult;
use std::{sync::Arc, time::Duration};
use tokio::sync::Mutex;
use tracing::{error, info, warn};

async fn insert_vehicle_tracker_location(
    db: &DatabaseConnection,
    timestamp: DateTime<Utc>,
    tracker_id: i32,
    lat: f64,
    lng: f64,
) -> Result<PgQueryResult, sqlx::Error> {
    let point: geo_types::Geometry<f64> = geo_types::Point::new(lat, lng).into();

    sqlx::query(
        "INSERT INTO vehicle_tracker_location (time, vehicle_tracker_id, point) VALUES ($1, $2, ST_SetSRID($3, 4326))",
    )
    .bind(timestamp)
    .bind(tracker_id)
    .bind(wkb::Encode(point))
    .execute(db.get_postgres_connection_pool())
    .await
}

async fn handle_h02_location(
    delivery: &Delivery,
    socket: &SocketIo,
    tracker_id: i32,
    db: &DatabaseConnection,
) {
    let parse_result: Result<h02::LocationMsg, serde_json::Error> =
        serde_json::from_slice(delivery.data.as_slice());

    match parse_result {
        Ok(decoded) => {
            let _ = insert_vehicle_tracker_location(
                db,
                decoded.timestamp,
                tracker_id,
                decoded.lat,
                decoded.lng,
            )
            .await;

            let position = PositionDto {
                lat: decoded.lat,
                lng: decoded.lng,
                tracker_id,
            };

            let _ = socket
                .of("/tracking")
                .expect("/tracking socket io namespace not available")
                .within(tracker_id.to_string())
                .emit("position", position);
        }
        Err(e) => {
            error!("failed to parse H02 location: {e}");
        }
    }
}

async fn on_tracker_event(
    tracker_cache: &Arc<Mutex<TrackerIdCache>>,
    delivery: Delivery,
    db: &DatabaseConnection,
    socket: &SocketIo,
) {
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

    let tracker_id: i32 = match tracker_cache.lock().await.get(imei).await {
        Some(id) => id,
        None => {
            warn!("tracker: {imei} doest not exist");
            return;
        }
    };

    let _ = handle_h02_location(&delivery, socket, tracker_id, db).await;
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

        let tracker_cache = Arc::new(Mutex::new(TrackerIdCache::new(db.clone())));

        let db_ref = &db;
        let socket_ref = &socket_io;
        let tracker_cache_ref = &tracker_cache;

        loop {
            tokio::time::sleep(Duration::from_secs(5)).await;
            info!("[RMQ] starting tracker positions consumer");

            // TODO: decide how to properly trace this
            // integrate with jaeger and context propagation
            let consume_end_result = rmq
                .consume(
                    TRACKER_EVENTS_QUEUE,
                    "api_tracker_events_consumer",
                    consume_options,
                    FieldTable::default(),
                    |delivery: Delivery| async move {
                        on_tracker_event(tracker_cache_ref, delivery, db_ref, socket_ref).await
                    },
                )
                .await;

            if let Err(error) = consume_end_result {
                error!("[RMQ] tracker positions consumer error {error}");
            }
        }
    });
}
