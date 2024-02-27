use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::Serialize;
use utoipa::ToSchema;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, ToSchema)]
#[schema(as = entity::sim_card::Model)]
#[sea_orm(table_name = "sim_card")]
#[serde(rename_all = "camelCase")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub created_at: DateTime<Utc>,
    pub phone_number: String,
    pub ssn: String,
    pub apn_address: String,
    pub apn_user: String,
    pub apn_password: String,
    pub pin: Option<String>,
    pub pin2: Option<String>,
    pub puk: Option<String>,
    pub puk2: Option<String>,
    pub organization_id: i32,
    pub vehicle_tracker_id: Option<i32>,
}

impl Entity {
    pub async fn find_by_id_and_org_id(
        id: i32,
        organization_id: i32,
        db: &DatabaseConnection,
    ) -> Result<Option<Model>, DbErr> {
        Self::find()
            .filter(Column::Id.eq(id))
            .filter(Column::OrganizationId.eq(organization_id))
            .one(db)
            .await
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
    #[sea_orm(
        belongs_to = "super::vehicle_tracker::Entity",
        from = "Column::VehicleTrackerId",
        to = "super::vehicle_tracker::Column::Id",
        on_update = "Cascade",
        on_delete = "SetNull"
    )]
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
