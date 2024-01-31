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
        let test_user = seeder::test_user(&transaction).await?;

        seeder::create_entities_for_org(&transaction, test_user.organization_id.unwrap()).await?;

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
