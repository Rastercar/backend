use strum::{Display, EnumIter, IntoEnumIterator};

/// All the tracker models that are supported by rastercar
#[derive(Debug, EnumIter, Display, Clone)]
pub enum TrackerModel {
    H02,
}

impl TrackerModel {
    /// Creates a string vector containing all the permissions in screaming snake case format
    pub fn to_string_vec() -> Vec<String> {
        TrackerModel::iter()
            .map(|e| e.to_string())
            .collect::<Vec<_>>()
    }
}
