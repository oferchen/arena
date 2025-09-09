use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Entitlements::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Entitlements::UserId).uuid().not_null())
                    .col(ColumnDef::new(Entitlements::SkuId).string().not_null())
                    .col(
                        ColumnDef::new(Entitlements::GrantedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::cust("NOW()")),
                    )
                    .primary_key(
                        Index::create()
                            .col(Entitlements::UserId)
                            .col(Entitlements::SkuId),
                    )
                    .to_owned(),
            )
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Entitlements::Table).to_owned())
            .await?;
        Ok(())
    }
}

#[derive(Iden)]
enum Entitlements {
    Table,
    UserId,
    SkuId,
    GrantedAt,
}
