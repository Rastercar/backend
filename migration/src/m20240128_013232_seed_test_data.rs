use sea_orm_migration::sea_orm::{entity::*, query::*};
use sea_orm_migration::{prelude::*, sea_orm::TransactionTrait};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        let transaction = db.begin().await?;

        // let test_master_user_access_level = fake_access_level(conn, true, None);

        // insert_into(user)
        //     .values((
        //         username.eq("test_master_user"),
        //         password.eq(hash_password(String::from("testmasteruser"))),
        //         email.eq("rastercar.tests.001@gmail.com"),
        //         email_verified.eq(true),
        //         access_level_id.eq(test_master_user_access_level.id),
        //     ))
        //     .get_result::<models::User>(conn)
        //     .unwrap();

        // user::ActiveModel {
        //     name: Set("Pear".to_owned()),
        //     ..Default::default()
        // }
        // .insert(db)
        // .await?;

        // Commit it
        transaction.commit().await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        todo!();
    }
}
