use crate::seeder;
use sea_orm_migration::{
    prelude::*,
    sea_orm::{prelude::*, EntityTrait, TransactionTrait},
};
use shared::entity::vehicle_tracker;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        let transaction = db.begin().await?;

        // maintain this order
        seeder::create_test_master_user(&transaction).await?;
        let test_user = seeder::create_test_user(&transaction).await?;

        seeder::create_entities_for_org(&transaction, test_user.organization_id.unwrap()).await?;

        for _ in 0..5 {
            seeder::root_user_with_user_org(&transaction).await.unwrap();
        }

        // change the first 10 tracker ids to values used by the tracker
        // sender mock so we can send mocked positions easily to all of them
        for i in 0..10 {
            vehicle_tracker::Entity::update_many()
                .col_expr(
                    vehicle_tracker::Column::Imei,
                    Expr::value(Value::String(Some(Box::new(format!(
                        "86723205114835{}",
                        i
                    ))))),
                )
                .filter(vehicle_tracker::Column::Id.eq(i))
                .exec(&transaction)
                .await?;
        }

        transaction.commit().await?;

        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Err(DbErr::Custom(String::from("cannot be reverted")))
    }
}
