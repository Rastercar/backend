pub use sea_orm_migration::prelude::*;

mod m20240125_133701_install_extensions;
mod m20240125_134615_init;
mod m20240125_135000_hypertable_tracker_last_location;
mod m20240125_135052_last_position_trigger;
mod m20240128_013232_seed_test_data;
mod seeder;
mod seeder_consts;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20240125_133701_install_extensions::Migration),
            Box::new(m20240125_134615_init::Migration),
            Box::new(m20240125_135000_hypertable_tracker_last_location::Migration),
            Box::new(m20240125_135052_last_position_trigger::Migration),
            Box::new(m20240128_013232_seed_test_data::Migration),
        ]
    }
}
