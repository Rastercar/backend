use convert_case::{Case, Casing};
use sea_orm::DeriveActiveEnum;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use strum::{Display, EnumIter, IntoEnumIterator};
use utoipa::ToSchema;

/// All the permissions available for the rastercar API
#[derive(Debug, EnumIter, Display, Clone)]
pub enum Permission {
    CreateUser,
    UpdateUser,
    DeleteUser,
    LogoffUser,
    ListUserSessions,

    CreateAccessLevel,
    UpdateAccessLevel,
    DeleteAccessLevel,

    CreateTracker,
    UpdateTracker,
    DeleteTracker,

    CreateVehicle,
    UpdateVehicle,
    DeleteVehicle,

    DeleteSimCard,
    UpdateSimCard,
    CreateSimCard,

    UpdateOrganization,
}

impl Permission {
    /// Creates a string vector containing all the permissions in screaming snake case format
    pub fn to_string_vec() -> Vec<String> {
        Permission::iter()
            .map(|e| e.to_string().to_case(Case::ScreamingSnake))
            .collect::<Vec<_>>()
    }
}

pub struct TrackerModelInfo {
    /// amount of sim cards that can be installed on a tracker
    pub sim_card_slots: u8,
}

/// All the tracker models that are supported by rastercar
///
/// also the native ENUM for the rastercar postgres database
#[derive(
    Eq,
    Clone,
    Debug,
    Display,
    EnumIter,
    ToSchema,
    Serialize,
    PartialEq,
    Deserialize,
    DeriveActiveEnum,
)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "tracker_model")]
pub enum TrackerModel {
    #[sea_orm(string_value = "H02")]
    H02,
}

impl TrackerModel {
    pub fn to_string_vec() -> Vec<String> {
        TrackerModel::iter()
            .map(|e| e.to_string())
            .collect::<Vec<_>>()
    }

    pub const fn get_info(self) -> TrackerModelInfo {
        match self {
            Self::H02 => TrackerModelInfo { sim_card_slots: 1 },
        }
    }
}

impl FromStr for TrackerModel {
    type Err = ();

    fn from_str(input: &str) -> Result<TrackerModel, Self::Err> {
        match input {
            "H02" => Ok(TrackerModel::H02),
            _ => Err(()),
        }
    }
}
