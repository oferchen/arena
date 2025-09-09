use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(AnalyticsRollups::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(AnalyticsRollups::Event).string().not_null())
                    .col(
                        ColumnDef::new(AnalyticsRollups::Bucket)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(AnalyticsRollups::Count)
                            .big_integer()
                            .not_null(),
                    )
                    .primary_key(
                        Index::create()
                            .col(AnalyticsRollups::Event)
                            .col(AnalyticsRollups::Bucket),
                    )
                    .to_owned(),
            )
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(AnalyticsRollups::Table).to_owned())
            .await?;
        Ok(())
    }
}

#[derive(Iden)]
enum AnalyticsRollups {
    Table,
    Event,
    Bucket,
    Count,
}
