use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::Serialize;
use utoipa::ToSchema;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, ToSchema)]
#[schema(as = entity::vehicle_tracker::Model)]
#[sea_orm(table_name = "vehicle_tracker")]
#[serde(rename_all = "camelCase")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub created_at: DateTime<Utc>,
    pub model: String,
    pub imei: String,
    pub organization_id: i32,
    pub vehicle_id: Option<i32>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::organization::Entity",
        from = "Column::OrganizationId",
        to = "super::organization::Column::Id",
        on_update = "Cascade",
        on_delete = "NoAction"
    )]
    Organization,
    #[sea_orm(has_many = "super::sim_card::Entity")]
    SimCard,
    #[sea_orm(
        belongs_to = "super::vehicle::Entity",
        from = "Column::VehicleId",
        to = "super::vehicle::Column::Id",
        on_update = "Cascade",
        on_delete = "SetNull"
    )]
    Vehicle,
    #[sea_orm(has_one = "super::vehicle_tracker_last_location::Entity")]
    VehicleTrackerLastLocation,
}

impl Related<super::organization::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Organization.def()
    }
}

impl Related<super::sim_card::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::SimCard.def()
    }
}

impl Related<super::vehicle::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Vehicle.def()
    }
}

impl Related<super::vehicle_tracker_last_location::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::VehicleTrackerLastLocation.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
