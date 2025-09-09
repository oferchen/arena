pub use sea_orm_migration::prelude::*;

mod m20240101_000001_create_users;
mod m20240101_000002_create_email_verifications;
mod m20240101_000003_create_email_otps;
mod m20240101_000004_create_user_totp;
mod m20240101_000005_create_recovery_codes;
mod m20240101_000006_create_sessions;
mod m20240101_000007_create_rate_limits;
mod m20240101_000008_create_leaderboard;
mod m20240101_000009_create_leaderboard_tables;
mod m20240101_000010_create_entitlements;
mod m20240101_000011_create_purchases;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20240101_000001_create_users::Migration),
            Box::new(m20240101_000002_create_email_verifications::Migration),
            Box::new(m20240101_000003_create_email_otps::Migration),
            Box::new(m20240101_000004_create_user_totp::Migration),
            Box::new(m20240101_000005_create_recovery_codes::Migration),
            Box::new(m20240101_000006_create_sessions::Migration),
            Box::new(m20240101_000007_create_rate_limits::Migration),
            Box::new(m20240101_000008_create_leaderboard::Migration),
            Box::new(m20240101_000009_create_leaderboard_tables::Migration),
            Box::new(m20240101_000010_create_entitlements::Migration),
            Box::new(m20240101_000011_create_purchases::Migration),
        ]
    }
}
