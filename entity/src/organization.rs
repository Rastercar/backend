use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "organization")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub created_at: DateTime<Utc>,
    pub name: String,
    pub blocked: bool,
    #[sea_orm(unique)]
    pub billing_email: String,
    pub billing_email_verified: bool,
    #[sea_orm(column_type = "Text", nullable)]
    pub confirm_billing_email_token: Option<String>,
    #[sea_orm(unique)]
    pub owner_id: Option<i32>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::access_level::Entity")]
    AccessLevel,
    #[sea_orm(has_many = "super::sim_card::Entity")]
    SimCard,
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::OwnerId",
        to = "super::user::Column::Id",
        on_update = "Cascade",
        on_delete = "SetNull"
    )]
    User,
    #[sea_orm(has_many = "super::vehicle::Entity")]
    Vehicle,
    #[sea_orm(has_many = "super::vehicle_tracker::Entity")]
    VehicleTracker,
}

impl Related<super::access_level::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::AccessLevel.def()
    }
}

impl Related<super::sim_card::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::SimCard.def()
    }
}

impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl Related<super::vehicle::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Vehicle.def()
    }
}

impl Related<super::vehicle_tracker::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::VehicleTracker.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
