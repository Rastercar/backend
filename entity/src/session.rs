use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;

use crate::user;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "session")]
pub struct Model {
    #[sea_orm(unique)]
    pub public_id: i32,
    #[sea_orm(
        primary_key,
        auto_increment = false,
        column_type = "Binary(BlobSize::Blob(None))"
    )]
    pub session_token: Vec<u8>,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub user_agent: String,
    #[sea_orm(column_type = "custom(\"inet\")", select_as = "text", save_as = "inet")]
    pub ip: String,
    pub user_id: i32,
}

impl Entity {
    pub async fn find_with_user_by_public_id(
        public_id: i32,
        db: &DatabaseConnection,
    ) -> Result<Option<(Model, user::Model)>, DbErr> {
        let res = user::Entity::find()
            .filter(Column::PublicId.eq(public_id))
            .find_also_related(Self)
            .one(db)
            .await?;

        if let Some((session, user_opt)) = res {
            if let Some(user) = user_opt {
                return Ok(Some((user, session)));
            }
        }

        Ok(None)
    }
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::UserId",
        to = "super::user::Column::Id",
        on_update = "NoAction",
        on_delete = "Cascade"
    )]
    User,
}

impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
