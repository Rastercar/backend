use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        // This is the trigger function which will be executed for each row of an INSERT or UPDATE.
        let statement = r#"
        ALTER TABLE vehicle_tracker_last_location SET (fillfactor=95);

        CREATE OR REPLACE FUNCTION create_last_pos_trigger_fn() RETURNS TRIGGER LANGUAGE PLPGSQL AS
              $BODY$
                  BEGIN
                      INSERT INTO vehicle_tracker_last_location (tracker_id, point, time) VALUES (NEW.tracker_id, NEW.point, NEW.time) 
                      ON CONFLICT (tracker_id) DO UPDATE SET 
                      point=NEW.point,
                      time=new.time;
                      RETURN NEW;
                  END
              $BODY$;
        "#;

        db.execute_unprepared(statement).await?;

        // With the trigger function created, actually assign it to the
        // vehicle_tracker_location table so that it will execute for each row
        let statement = r#"
        CREATE TRIGGER create_last_position_trigger
        BEFORE INSERT OR UPDATE ON vehicle_tracker_location
        FOR EACH ROW EXECUTE PROCEDURE create_last_pos_trigger_fn();
        "#;

        db.execute_unprepared(statement).await?;

        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Err(DbErr::Custom(String::from("cannot be reverted")))
    }
}
