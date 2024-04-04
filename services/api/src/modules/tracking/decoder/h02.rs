use super::super::utils;
use crate::modules::tracking::dto::PositionDto;
use lapin::message::Delivery;
use sea_orm::DatabaseConnection;
use socketioxide::SocketIo;
use tracing::error;

#[tracing::instrument(skip_all)]
pub async fn handle_location(
    delivery: &Delivery,
    socket: &SocketIo,
    tracker_id: i32,
    db: &DatabaseConnection,
) {
    let parse_result: Result<shared::dto::decoder::h02::LocationMsg, serde_json::Error> =
        serde_json::from_slice(delivery.data.as_slice());

    match parse_result {
        Ok(decoded) => {
            let _ = utils::insert_vehicle_tracker_location(
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
                timestamp: decoded.timestamp,
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
