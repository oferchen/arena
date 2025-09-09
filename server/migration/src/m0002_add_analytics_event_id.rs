use sea_orm_migration::prelude::*;
use sea_orm::{ConnectionTrait, Statement};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Add id column
        manager
            .get_connection()
            .execute(Statement::from_string(
                manager.get_database_backend(),
                "ALTER TABLE analytics_events ADD COLUMN id BIGSERIAL",
            ))
            .await?;
        // Drop old primary key on ts
        manager
            .get_connection()
            .execute(Statement::from_string(
                manager.get_database_backend(),
                "ALTER TABLE analytics_events DROP CONSTRAINT analytics_events_pkey",
            ))
            .await?;
        // Add new primary key on id
        manager
            .get_connection()
            .execute(Statement::from_string(
                manager.get_database_backend(),
                "ALTER TABLE analytics_events ADD PRIMARY KEY (id)",
            ))
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop primary key on id
        manager
            .get_connection()
            .execute(Statement::from_string(
                manager.get_database_backend(),
                "ALTER TABLE analytics_events DROP CONSTRAINT analytics_events_pkey",
            ))
            .await?;
        // Add primary key back on ts
        manager
            .get_connection()
            .execute(Statement::from_string(
                manager.get_database_backend(),
                "ALTER TABLE analytics_events ADD PRIMARY KEY (ts)",
            ))
            .await?;
        // Remove id column
        manager
            .get_connection()
            .execute(Statement::from_string(
                manager.get_database_backend(),
                "ALTER TABLE analytics_events DROP COLUMN id",
            ))
            .await?;
        Ok(())
    }
}
