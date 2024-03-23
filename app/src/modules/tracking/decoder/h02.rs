use super::super::utils;
use crate::modules::tracking::dto::PositionDto;
use chrono::{DateTime, Utc};
use lapin::message::Delivery;
use sea_orm::DatabaseConnection;
use serde::Deserialize;
use socketioxide::SocketIo;
use tracing::error;

#[tracing::instrument(skip_all)]
pub async fn handle_location(
    delivery: &Delivery,
    socket: &SocketIo,
    tracker_id: i32,
    db: &DatabaseConnection,
) {
    let parse_result: Result<LocationMsg, serde_json::Error> =
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

#[derive(Deserialize)]
pub struct LocationMsg {
    /// latitude (90 to -90) in decimal degrees
    pub lat: f64,

    /// longitude (180 to -180) in decimal degrees
    pub lng: f64,

    /// speed in km/h
    pub speed: f64,

    /// info about vehicle / tracker status
    pub status: Status,

    /// direction in degrees (0 degrees = north, 180 = s)
    pub direction: i32,

    /// vehicle date and time sent by the tracker
    pub timestamp: DateTime<Utc>,
}

#[derive(Deserialize)]
pub struct Status {
    pub temperature_alarm: bool,
    pub three_times_pass_error_alarm: bool,
    pub gprs_occlusion_alarm: bool,
    pub oil_and_engine_cut_off: bool,
    pub storage_battery_removal_state: bool,
    pub high_level_sensor1: bool,
    pub high_level_sensor2: bool,
    pub low_level_sensor1_bond_strap: bool,
    pub gps_receiver_fault_alarm: bool,
    pub analog_quantity_transfinit_alarm: bool,
    pub sos_alarm: bool,
    pub host_powered_by_backup_battery: bool,
    pub storage_battery_removed: bool,
    pub open_circuit_for_gps_antenna: bool,
    pub short_circuit_for_gps_antenna: bool,
    pub low_level_sensor2_bond_strap: bool,
    pub door_open: bool,
    pub vehicle_fortified: bool,
    pub acc: bool,
    pub engine: bool,
    pub custom_alarm: bool,
    pub overspeed: bool,
    pub theft_alarm: bool,
    pub roberry_alarm: bool,
    pub overspeed_alarm: bool,
    pub illegal_ignition_alarm: bool,
    pub no_entry_cross_border_alarm_in: bool,
    pub gps_antenna_open_circuit_alarm: bool,
    pub gps_antenna_short_circuit_alarm: bool,
    pub no_entry_cross_border_alarm_out: bool,
}
