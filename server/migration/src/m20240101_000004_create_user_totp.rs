use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(UserTotp::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(UserTotp::UserId)
                            .big_integer()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(UserTotp::Secret).text().not_null())
                    .col(
                        ColumnDef::new(UserTotp::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::cust("NOW()")),
                    )
                    .col(
                        ColumnDef::new(UserTotp::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::cust("NOW()")),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_user_totp_user_id")
                            .from(UserTotp::Table, UserTotp::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(UserTotp::Table).to_owned())
            .await?;
        Ok(())
    }
}

#[derive(Iden)]
enum UserTotp {
    Table,
    UserId,
    Secret,
    CreatedAt,
    UpdatedAt,
}

#[derive(Iden)]
enum Users {
    Table,
    Id,
}
