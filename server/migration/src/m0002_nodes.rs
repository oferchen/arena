use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Nodes::Table)
                    .rename_column(Nodes::CreatedAt, Nodes::LastSeen)
                    .add_column(
                        ColumnDef::new(Nodes::Info)
                            .json_binary()
                            .not_null()
                            .default(Expr::cust("'{}'::jsonb"))
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Nodes::Table)
                    .drop_column(Nodes::Info)
                    .rename_column(Nodes::LastSeen, Nodes::CreatedAt)
                    .to_owned(),
            )
            .await
    }
}

#[derive(Iden)]
enum Nodes {
    Table,
    Id,
    Region,
    CreatedAt,
    LastSeen,
    Info,
}

