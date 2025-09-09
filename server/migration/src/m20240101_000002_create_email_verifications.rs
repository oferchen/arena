use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(EmailVerifications::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(EmailVerifications::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(EmailVerifications::UserId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(EmailVerifications::Token)
                            .text()
                            .not_null()
                            .unique_key(),
                    )
                    .col(
                        ColumnDef::new(EmailVerifications::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::cust("NOW()")),
                    )
                    .col(
                        ColumnDef::new(EmailVerifications::ExpiresAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_email_verifications_user_id")
                            .from(EmailVerifications::Table, EmailVerifications::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_email_verifications_user_id")
                    .table(EmailVerifications::Table)
                    .col(EmailVerifications::UserId)
                    .to_owned(),
            )
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(EmailVerifications::Table).to_owned())
            .await?;
        Ok(())
    }
}

#[derive(Iden)]
enum EmailVerifications {
    Table,
    Id,
    UserId,
    Token,
    CreatedAt,
    ExpiresAt,
}

#[derive(Iden)]
enum Users {
    Table,
    Id,
}
