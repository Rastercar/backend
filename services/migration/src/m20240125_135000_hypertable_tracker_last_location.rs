use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        // for reasoning about the indexes, see:
        // https://www.timescale.com/blog/select-the-most-recent-record-of-many-items-with-postgresql/
        db.execute_unprepared(
            "CREATE INDEX ix_time ON vehicle_tracker_location (time DESC);

             CREATE INDEX ix_time_vehicle_tracker_id ON vehicle_tracker_location (vehicle_tracker_id, time DESC);

             SELECT create_hypertable('vehicle_tracker_location','time');",
        )
        .await?;

        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Err(DbErr::Custom(String::from("cannot be reverted")))
    }
}
