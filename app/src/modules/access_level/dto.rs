use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use validator::Validate;

#[derive(Deserialize, IntoParams, Validate)]
#[serde(rename_all = "camelCase")]
#[into_params(parameter_in = Query)]
pub struct ListAccessLevelsDto {
    /// Search by name
    pub name: Option<String>,
}

#[derive(Serialize, Clone, ToSchema)]
#[serde(rename_all = "camelCase")]
#[schema(as = access_level::dto::AccessLevelDto)]
pub struct AccessLevelDto {
    pub id: i32,
    pub created_at: DateTime<Utc>,
    pub name: String,
    pub description: String,
    pub is_fixed: bool,
    pub permissions: Vec<String>,
}

impl From<entity::access_level::Model> for AccessLevelDto {
    fn from(m: entity::access_level::Model) -> Self {
        Self {
            id: m.id,
            created_at: m.created_at,
            name: m.name,
            description: m.description,
            is_fixed: m.is_fixed,
            permissions: m.permissions,
        }
    }
}
