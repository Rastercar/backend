use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "access_level")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub created_at: DateTime<Utc>,
    pub name: String,
    #[sea_orm(column_type = "Text")]
    pub description: String,
    pub is_fixed: bool,
    pub permissions: Vec<String>,
    pub organization_id: Option<i32>,
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
        on_delete = "SetNull"
    )]
    Organization,
    #[sea_orm(has_many = "super::user::Entity")]
    User,
}

impl Related<super::organization::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Organization.def()
    }
}

impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
