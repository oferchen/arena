use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(RecoveryCodes::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(RecoveryCodes::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(RecoveryCodes::UserId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(RecoveryCodes::Code)
                            .text()
                            .not_null()
                            .unique_key(),
                    )
                    .col(ColumnDef::new(RecoveryCodes::UsedAt).timestamp_with_time_zone())
                    .col(
                        ColumnDef::new(RecoveryCodes::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::cust("NOW()")),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_recovery_codes_user_id")
                            .from(RecoveryCodes::Table, RecoveryCodes::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_recovery_codes_user_id")
                    .table(RecoveryCodes::Table)
                    .col(RecoveryCodes::UserId)
                    .to_owned(),
            )
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(RecoveryCodes::Table).to_owned())
            .await?;
        Ok(())
    }
}

#[derive(Iden)]
enum RecoveryCodes {
    Table,
    Id,
    UserId,
    Code,
    UsedAt,
    CreatedAt,
}

#[derive(Iden)]
enum Users {
    Table,
    Id,
}
