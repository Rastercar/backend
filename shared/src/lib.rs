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

/// All the tracker models that are supported by rastercar
#[derive(Debug, EnumIter, Display, Clone)]
pub enum TrackerModel {
    H02,
}

impl TrackerModel {
    pub fn to_string_vec() -> Vec<String> {
        TrackerModel::iter()
            .map(|e| e.to_string())
            .collect::<Vec<_>>()
    }
}
