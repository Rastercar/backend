use super::utils;
use crate::protocols::common::{Decoded, Protocol, TrackerEvent};
use chrono::{DateTime, Utc};
use hex;
use shared::dto::decoder::h02::{LocationMsg, Status};

struct LocationPackets<'a> {
    imei: &'a str,
    _cmd: &'a str,
    time: &'a str,
    data_valid_bit: &'a str,
    lat: &'a str,
    lat_symbol: &'a str,
    lng: &'a str,
    lng_symbol: &'a str,
    speed: &'a str,
    direction_degrees: &'a str,
    date: &'a str,
    status: &'a str,
}

impl LocationPackets<'_> {
    fn parse_direction(&self) -> Result<i32, &str> {
        self.direction_degrees
            .parse::<i32>()
            .or(Err("failed to parse direction degrees to int"))
    }

    fn parse_speed(&self) -> Result<f64, &str> {
        let s = self
            .speed
            .parse::<f64>()
            .or(Err("failed to parse speed to float in km/h"))?;

        // convert knots/h to km/h
        Ok(s * 1.852)
    }

    fn parse_lat(&self) -> Result<f64, String> {
        let mut lat = utils::str_to_lat(self.lat)?;

        if self.lat_symbol == "S" || self.lat_symbol == "s" {
            lat *= -1.0
        }

        Ok(lat)
    }

    fn parse_lng(&self) -> Result<f64, String> {
        let mut lng = utils::str_to_lng(self.lng)?;

        if self.lng_symbol == "W" || self.lng_symbol == "w" {
            lng *= -1.0
        }

        Ok(lng)
    }

    fn parse_status(&self) -> Result<Status, String> {
        let status_bytes = hex::decode(self.status).or(Err("failed to parse status bytes"))?;

        if status_bytes.len() < 4 {
            return Err("cannot decoded status bytes, as it does not contain 4 bytes".to_string());
        }

        let mut binary_str = "".to_string();

        for byte in status_bytes {
            binary_str.push_str(&format!("{:b}", byte));
        }

        let bin_chars: Vec<char> = binary_str.chars().collect();

        let b = |i: usize| -> bool { bin_chars[i] == '1' };

        Ok(Status {
            // byte 1
            temperature_alarm: b(0),
            three_times_pass_error_alarm: b(1),
            gprs_occlusion_alarm: b(2),
            oil_and_engine_cut_off: b(3),
            storage_battery_removal_state: b(4),
            high_level_sensor1: b(5),
            high_level_sensor2: b(6),
            low_level_sensor1_bond_strap: b(7),

            // byte 2
            gps_receiver_fault_alarm: b(8),
            analog_quantity_transfinit_alarm: b(9),
            sos_alarm: b(10),
            host_powered_by_backup_battery: b(11),
            storage_battery_removed: b(12),
            open_circuit_for_gps_antenna: b(13),
            short_circuit_for_gps_antenna: b(14),
            low_level_sensor2_bond_strap: b(15),

            // byte 3
            door_open: b(16),
            vehicle_fortified: b(17),
            acc: b(18),
            // 19: reserved
            // 20: reserved
            engine: b(21),
            custom_alarm: b(22),
            overspeed: b(23),

            // byte 4
            theft_alarm: b(24),
            roberry_alarm: b(25),
            overspeed_alarm: b(26),
            illegal_ignition_alarm: b(27),
            no_entry_cross_border_alarm_in: b(28),
            gps_antenna_open_circuit_alarm: b(29),
            gps_antenna_short_circuit_alarm: b(30),
            no_entry_cross_border_alarm_out: b(31),
        })
    }

    fn decode(&self) -> Result<LocationMsg, String> {
        if self.data_valid_bit != "A" {
            return Err("invalid location data (data valid bit != A)".to_string());
        }

        Ok(LocationMsg {
            lat: self.parse_lat()?,
            lng: self.parse_lng()?,
            speed: self.parse_speed()?,
            status: self.parse_status()?,
            direction: self.parse_direction()?,
            timestamp: self.parse_timestamp()?,
        })
    }

    fn parse_timestamp(&self) -> Result<DateTime<Utc>, String> {
        if self.date.len() < 6 {
            return Err("cannot parse date outside expected ddmmyy format".to_string());
        }

        if self.time.len() < 6 {
            return Err("cannot parse time outside expected hhmmss format".to_string());
        }

        // example: "2014-11-28T12:00:09Z"
        let iso_timestamp = [
            "20",
            &self.date[4..6],
            "-",
            &self.date[2..4],
            "-",
            &self.date[..2],
            "T",
            &self.time[..2],
            ":",
            &self.time[2..4],
            ":",
            &self.time[4..6],
            "Z",
        ]
        .concat();

        iso_timestamp
            .parse::<DateTime<Utc>>()
            .or(Err(format!("failed to parse date time {iso_timestamp}")))
    }
}

impl TryFrom<Vec<&str>> for Decoded<LocationMsg> {
    type Error = String;

    fn try_from(parts: Vec<&str>) -> Result<Self, Self::Error> {
        if parts.len() < 12 {
            return Err("incomplete location message".to_string());
        }

        let packets = LocationPackets {
            imei: parts[0],
            _cmd: parts[1],
            time: parts[2],
            data_valid_bit: parts[3],
            lat: parts[4],
            lat_symbol: parts[5],
            lng: parts[6],
            lng_symbol: parts[7],
            speed: parts[8],
            direction_degrees: parts[9],
            date: parts[10],
            status: parts[11],
        };

        Ok(Decoded {
            data: packets.decode()?,
            imei: packets.imei.to_string(),
            response: None,
            protocol: Protocol::H02,
            event_type: TrackerEvent::Location,
        })
    }
}
