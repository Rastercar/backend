use sea_orm_migration::{prelude::*, sea_orm::TransactionTrait};

use crate::seeder;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        let transaction = db.begin().await?;

        // maintain this order
        seeder::test_master_user(&transaction).await?;
        seeder::test_user(&transaction).await?;

        for _ in 1..20 {
            seeder::root_user_with_user_org(&transaction).await?;
        }

        transaction.commit().await?;

        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        todo!();
    }
}
