use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(MailOutbox::Table)
                    .add_column(
                        ColumnDef::new(MailOutbox::SentAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .add_column(ColumnDef::new(MailOutbox::Error).string().null())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(MailOutbox::Table)
                    .drop_column(MailOutbox::SentAt)
                    .drop_column(MailOutbox::Error)
                    .to_owned(),
            )
            .await
    }
}

#[derive(Iden)]
enum MailOutbox {
    Table,
    SentAt,
    Error,
}
