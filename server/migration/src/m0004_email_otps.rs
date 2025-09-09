use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(EmailOtps::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(EmailOtps::EmailHash)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(EmailOtps::Code).string().not_null())
                    .col(
                        ColumnDef::new(EmailOtps::ExpiresAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(EmailOtps::Table).to_owned())
            .await?;
        Ok(())
    }
}

#[derive(Iden)]
enum EmailOtps {
    Table,
    EmailHash,
    Code,
    ExpiresAt,
}
