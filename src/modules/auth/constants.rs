use convert_case::{Case, Casing};
use strum::{Display, EnumIter, IntoEnumIterator};

#[derive(Debug, EnumIter, Display)]
pub enum Permission {
    CreateVehicle,
    UpdateVehicle,
    DeleteVehicle,
}

impl Permission {
    /// Creates a string vector containing all the permissions in screaming snake case format
    pub fn to_string_vec() -> Vec<String> {
        Permission::iter()
            .map(|e| e.to_string().to_case(Case::ScreamingSnake))
            .collect::<Vec<_>>()
    }
}
