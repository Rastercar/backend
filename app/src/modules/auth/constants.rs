use convert_case::{Case, Casing};
use strum::{Display, EnumIter, IntoEnumIterator};

/// All the permissions available for the rastercar API
#[derive(Debug, EnumIter, Display, Clone)]
pub enum Permission {
    CreateTracker,
    CreateVehicle,
    UpdateVehicle,
    DeleteVehicle,
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
