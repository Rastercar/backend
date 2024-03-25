use super::{traits::QueryableByIdAndOrgId, vehicle_tracker};
use chrono::{DateTime, Utc};
use sea_orm::{entity::prelude::*, QuerySelect};
use serde::Serialize;
use utoipa::ToSchema;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, ToSchema)]
#[schema(as = entity::vehicle::Model)]
#[sea_orm(table_name = "vehicle")]
#[serde(rename_all = "camelCase")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub created_at: DateTime<Utc>,
    pub plate: String,
    pub photo: Option<String>,
    pub model_year: Option<i16>,
    pub fabrication_year: Option<i16>,
    pub chassis_number: Option<String>,
    pub brand: Option<String>,
    pub model: Option<String>,
    pub color: Option<String>,
    pub additional_info: Option<String>,
    pub organization_id: i32,
}

impl QueryableByIdAndOrgId for Entity {
    type Model = Model;

    async fn find_by_id_and_org_id(
        id: i32,
        org_id: i32,
        db: &DatabaseConnection,
    ) -> Result<Option<Model>, DbErr> {
        Self::find()
            .filter(Column::Id.eq(id))
            .filter(Column::OrganizationId.eq(org_id))
            .one(db)
            .await
    }
}

impl Entity {
    pub async fn get_associated_tracker_count(
        id: i32,
        db: &DatabaseConnection,
    ) -> Result<i64, DbErr> {
        let cnt = vehicle_tracker::Entity::find()
            .select_only()
            .column_as(vehicle_tracker::Column::Id.count(), "count")
            .filter(vehicle_tracker::Column::VehicleId.eq(id))
            .into_tuple()
            .one(db)
            .await?
            .unwrap_or(0);

        Ok(cnt)
    }
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
    #[sea_orm(has_many = "super::vehicle_tracker::Entity")]
    VehicleTracker,
}

impl Related<super::organization::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Organization.def()
    }
}

impl Related<super::vehicle_tracker::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::VehicleTracker.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
