pub use sea_orm_migration::prelude::*;

mod m0001_init;
mod m0002_add_analytics_event_id;
mod m0003_create_leaderboard_tables;
mod m0004_email_otps;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m0001_init::Migration),
            Box::new(m0002_add_analytics_event_id::Migration),
            Box::new(m0003_create_leaderboard_tables::Migration),
            Box::new(m0004_email_otps::Migration),
        ]
    }
}
