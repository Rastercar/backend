use std::str::FromStr;

use convert_case::{Case, Casing};
use strum::{Display, EnumIter, IntoEnumIterator};

/// All the permissions available for the rastercar API
#[derive(Debug, EnumIter, Display, Clone)]
pub enum Permission {
    CreateTracker,
    UpdateTracker,
    DeleteTracker,

    CreateVehicle,
    UpdateVehicle,
    DeleteVehicle,

    DeleteSimCard,
    UpdateSimCard,

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
#[derive(EnumIter, Display, Clone)]
pub enum TrackerModel {
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
