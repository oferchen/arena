use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(AnalyticsEvents::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(AnalyticsEvents::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(AnalyticsEvents::Name).string().not_null())
                    .col(ColumnDef::new(AnalyticsEvents::Data).string().null())
                    .col(
                        ColumnDef::new(AnalyticsEvents::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::cust("NOW()")),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_analytics_events_created_at")
                    .table(AnalyticsEvents::Table)
                    .col(AnalyticsEvents::CreatedAt)
                    .to_owned(),
            )
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(AnalyticsEvents::Table).to_owned())
            .await?;
        Ok(())
    }
}

#[derive(Iden)]
enum AnalyticsEvents {
    Table,
    Id,
    Name,
    Data,
    CreatedAt,
}
