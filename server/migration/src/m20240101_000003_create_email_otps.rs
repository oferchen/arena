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
                        ColumnDef::new(EmailOtps::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(EmailOtps::UserId).big_integer().not_null())
                    .col(ColumnDef::new(EmailOtps::OtpCode).text().not_null())
                    .col(
                        ColumnDef::new(EmailOtps::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::cust("NOW()")),
                    )
                    .col(
                        ColumnDef::new(EmailOtps::ExpiresAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_email_otps_user_id")
                            .from(EmailOtps::Table, EmailOtps::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_email_otps_user_id")
                    .table(EmailOtps::Table)
                    .col(EmailOtps::UserId)
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("uq_email_otps_user_id_code")
                    .table(EmailOtps::Table)
                    .col(EmailOtps::UserId)
                    .col(EmailOtps::OtpCode)
                    .unique()
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
    Id,
    UserId,
    OtpCode,
    CreatedAt,
    ExpiresAt,
}

#[derive(Iden)]
enum Users {
    Table,
    Id,
}
