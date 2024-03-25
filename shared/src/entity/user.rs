use super::traits::QueryableByIdAndOrgId;
use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "user")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,

    pub created_at: DateTime<Utc>,

    #[sea_orm(unique)]
    pub username: String,

    #[sea_orm(unique)]
    pub email: String,

    pub email_verified: bool,

    pub password: String,

    /// JWT to be used to reset the user password
    ///
    /// note: this is stored in the database because this token needs to be one time
    /// use only and a simple solution is to clear this column after the token is used
    #[sea_orm(column_type = "Text", nullable, unique)]
    pub reset_password_token: Option<String>,

    /// JWT to be used to confirm the user email address
    ///
    /// note: this is stored in the database because this token needs to be one time
    /// use only and a simple solution is to clear this column after the token is used
    #[sea_orm(column_type = "Text", nullable, unique)]
    pub confirm_email_token: Option<String>,

    pub profile_picture: Option<String>,

    #[sea_orm(column_type = "Text", nullable)]
    pub description: Option<String>,

    pub organization_id: Option<i32>,

    pub access_level_id: i32,
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

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::access_level::Entity",
        from = "Column::AccessLevelId",
        to = "super::access_level::Column::Id",
        on_update = "Cascade",
        on_delete = "NoAction"
    )]
    AccessLevel,
    #[sea_orm(
        belongs_to = "super::organization::Entity",
        from = "Column::OrganizationId",
        to = "super::organization::Column::Id",
        on_update = "Cascade",
        on_delete = "NoAction"
    )]
    Organization,
    #[sea_orm(has_many = "super::session::Entity")]
    Session,
}

impl Related<super::access_level::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::AccessLevel.def()
    }
}

impl Related<super::organization::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Organization.def()
    }
}

impl Related<super::session::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Session.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
