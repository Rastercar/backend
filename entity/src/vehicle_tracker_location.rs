use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "vehicle_tracker_location")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub time: DateTime<Utc>,
    #[sea_orm(primary_key, auto_increment = false)]
    pub tracker_id: i32,
    #[sea_orm(column_type = "custom(\"geometry\")")]
    pub point: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
