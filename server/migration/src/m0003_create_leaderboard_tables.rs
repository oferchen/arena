use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Runs::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Runs::Id).uuid().not_null().primary_key())
                    .col(ColumnDef::new(Runs::Leaderboard).uuid().not_null())
                    .col(ColumnDef::new(Runs::PlayerId).string().not_null())
                    .col(ColumnDef::new(Runs::ReplayPath).string().not_null())
                    .col(
                        ColumnDef::new(Runs::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::cust("NOW()")),
                    )
                    .col(
                        ColumnDef::new(Runs::Flagged)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(
                        ColumnDef::new(Runs::ReplayIndex)
                            .big_integer()
                            .not_null()
                            .default(0),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_runs_leaderboard")
                            .from(Runs::Table, Runs::Leaderboard)
                            .to(Leaderboards::Table, Leaderboards::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_runs_player")
                            .from(Runs::Table, Runs::PlayerId)
                            .to(Players::Table, Players::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_runs_leaderboard")
                    .table(Runs::Table)
                    .col(Runs::Leaderboard)
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_runs_player")
                    .table(Runs::Table)
                    .col(Runs::PlayerId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Scores::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Scores::Id).uuid().not_null().primary_key())
                    .col(ColumnDef::new(Scores::Run).uuid().not_null())
                    .col(ColumnDef::new(Scores::Leaderboard).uuid().not_null())
                    .col(ColumnDef::new(Scores::PlayerId).string().not_null())
                    .col(ColumnDef::new(Scores::Points).integer().not_null())
                    .col(
                        ColumnDef::new(Scores::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::cust("NOW()")),
                    )
                    .col(
                        ColumnDef::new(Scores::Verified)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_scores_run")
                            .from(Scores::Table, Scores::Run)
                            .to(Runs::Table, Runs::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_scores_player")
                            .from(Scores::Table, Scores::PlayerId)
                            .to(Players::Table, Players::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_scores_leaderboard")
                    .table(Scores::Table)
                    .col(Scores::Leaderboard)
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_scores_player")
                    .table(Scores::Table)
                    .col(Scores::PlayerId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Purchases::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Purchases::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Purchases::PlayerId).string().not_null())
                    .col(ColumnDef::new(Purchases::Sku).string().not_null())
                    .col(
                        ColumnDef::new(Purchases::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::cust("NOW()")),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_purchases_player")
                            .from(Purchases::Table, Purchases::PlayerId)
                            .to(Players::Table, Players::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_purchases_player")
                    .table(Purchases::Table)
                    .col(Purchases::PlayerId)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Scores::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Runs::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Purchases::Table).to_owned())
            .await?;
        Ok(())
    }
}

#[derive(Iden)]
enum Players {
    Table,
    Id,
}

#[derive(Iden)]
enum Leaderboards {
    Table,
    Id,
}

#[derive(Iden)]
enum Runs {
    Table,
    Id,
    Leaderboard,
    PlayerId,
    ReplayPath,
    CreatedAt,
    Flagged,
    ReplayIndex,
}

#[derive(Iden)]
enum Scores {
    Table,
    Id,
    Run,
    Leaderboard,
    PlayerId,
    Points,
    CreatedAt,
    Verified,
}

#[derive(Iden)]
enum Purchases {
    Table,
    Id,
    PlayerId,
    Sku,
    CreatedAt,
}
