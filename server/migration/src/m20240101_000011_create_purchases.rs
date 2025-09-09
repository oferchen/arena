use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Purchases::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Purchases::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Purchases::UserId).uuid().not_null())
                    .col(ColumnDef::new(Purchases::SkuId).string().not_null())
                    .col(
                        ColumnDef::new(Purchases::PurchasedAt)
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
                    .name("idx_purchases_user_id")
                    .table(Purchases::Table)
                    .col(Purchases::UserId)
                    .to_owned(),
            )
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Purchases::Table).to_owned())
            .await?;
        Ok(())
    }
}

#[derive(Iden)]
enum Purchases {
    Table,
    Id,
    UserId,
    SkuId,
    PurchasedAt,
}
