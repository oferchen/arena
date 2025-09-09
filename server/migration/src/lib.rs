pub use sea_orm_migration::prelude::*;

mod m20240101_000007_create_rate_limits;
mod m20240101_000008_create_leaderboard;
mod m20240101_000009_create_leaderboard_tables;
mod m20240101_000010_create_entitlements;
mod m20240101_000011_create_purchases;
mod m20240101_000012_create_analytics_events;
mod m20240101_000013_create_analytics_rollups;
mod m20240101_000014_create_players;
mod m20240101_000015_create_email_otps;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20240101_000007_create_rate_limits::Migration),
            Box::new(m20240101_000008_create_leaderboard::Migration),
            Box::new(m20240101_000009_create_leaderboard_tables::Migration),
            Box::new(m20240101_000010_create_entitlements::Migration),
            Box::new(m20240101_000011_create_purchases::Migration),
            Box::new(m20240101_000012_create_analytics_events::Migration),
            Box::new(m20240101_000013_create_analytics_rollups::Migration),
            Box::new(m20240101_000014_create_players::Migration),
            Box::new(m20240101_000015_create_email_otps::Migration),
        ]
    }
}
