use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Leaderboards::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Leaderboards::Id)
                            .text()
                            .not_null()
                            .primary_key(),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_table(
                Table::create()
                    .table(Runs::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Runs::Id).text().not_null().primary_key())
                    .col(ColumnDef::new(Runs::LeaderboardId).text().not_null())
                    .col(ColumnDef::new(Runs::PlayerId).text().not_null())
                    .col(ColumnDef::new(Runs::ReplayPath).text().not_null())
                    .col(ColumnDef::new(Runs::CreatedAt).text().not_null())
                    .col(
                        ColumnDef::new(Runs::Flagged)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_table(
                Table::create()
                    .table(Scores::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Scores::Id).text().not_null().primary_key())
                    .col(ColumnDef::new(Scores::RunId).text().not_null())
                    .col(ColumnDef::new(Scores::PlayerId).text().not_null())
                    .col(ColumnDef::new(Scores::Points).integer().not_null())
                    .col(ColumnDef::new(Scores::CreatedAt).text().not_null())
                    .col(
                        ColumnDef::new(Scores::Verified)
                            .integer()
                            .not_null()
                            .default(0),
                    )
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
            .drop_table(Table::drop().table(Leaderboards::Table).to_owned())
            .await?;
        Ok(())
    }
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
    LeaderboardId,
    PlayerId,
    ReplayPath,
    CreatedAt,
    Flagged,
}

#[derive(Iden)]
enum Scores {
    Table,
    Id,
    RunId,
    PlayerId,
    Points,
    CreatedAt,
    Verified,
}
