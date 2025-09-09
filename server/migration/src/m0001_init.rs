use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // players
        manager
            .create_table(
                Table::create()
                    .table(Players::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Players::Id).uuid().not_null().primary_key())
                    .col(ColumnDef::new(Players::Handle).string().not_null())
                    .col(ColumnDef::new(Players::Region).string().not_null())
                    .col(
                        ColumnDef::new(Players::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::cust("NOW()")),
                    )
                    .to_owned(),
            )
            .await?;
        // login_tokens
        manager
            .create_table(
                Table::create()
                    .table(LoginTokens::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(LoginTokens::Token).string().not_null().primary_key())
                    .col(ColumnDef::new(LoginTokens::Player).string().not_null())
                    .col(
                        ColumnDef::new(LoginTokens::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::cust("NOW()")),
                    )
                    .to_owned(),
            )
            .await?;
        // leaderboards
        manager
            .create_table(
                Table::create()
                    .table(Leaderboards::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Leaderboards::Id).uuid().not_null().primary_key())
                    .to_owned(),
            )
            .await?;
        // runs
        manager
            .create_table(
                Table::create()
                    .table(Runs::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Runs::Id).uuid().not_null().primary_key())
                    .col(ColumnDef::new(Runs::Leaderboard).uuid().not_null())
                    .col(ColumnDef::new(Runs::Player).uuid().not_null())
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
                    .col(ColumnDef::new(Runs::ReplayIndex).big_integer().not_null().default(0))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_runs_leaderboard")
                            .from(Runs::Table, Runs::Leaderboard)
                            .to(Leaderboards::Table, Leaderboards::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;
        // scores
        manager
            .create_table(
                Table::create()
                    .table(Scores::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Scores::Id).uuid().not_null().primary_key())
                    .col(ColumnDef::new(Scores::Run).uuid().not_null())
                    .col(ColumnDef::new(Scores::Leaderboard).uuid().not_null())
                    .col(ColumnDef::new(Scores::Player).uuid().not_null())
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
                    .to_owned(),
            )
            .await?;
        // entitlements
        manager
            .create_table(
                Table::create()
                    .table(Entitlements::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Entitlements::Player).uuid().not_null())
                    .col(ColumnDef::new(Entitlements::Sku).string().not_null())
                    .col(
                        ColumnDef::new(Entitlements::GrantedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::cust("NOW()")),
                    )
                    .primary_key(
                        Index::create()
                            .col(Entitlements::Player)
                            .col(Entitlements::Sku),
                    )
                    .to_owned(),
            )
            .await?;
        // purchases
        manager
            .create_table(
                Table::create()
                    .table(Purchases::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Purchases::Id).uuid().not_null().primary_key())
                    .col(ColumnDef::new(Purchases::Player).uuid().not_null())
                    .col(ColumnDef::new(Purchases::Sku).string().not_null())
                    .col(
                        ColumnDef::new(Purchases::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::cust("NOW()")),
                    )
                    .to_owned(),
            )
            .await?;
        // levels
        manager
            .create_table(
                Table::create()
                    .table(Levels::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Levels::Id).uuid().not_null().primary_key())
                    .col(ColumnDef::new(Levels::Name).string().not_null())
                    .col(ColumnDef::new(Levels::Data).text().not_null())
                    .to_owned(),
            )
            .await?;
        // analytics_events
        manager
            .create_table(
                Table::create()
                    .table(AnalyticsEvents::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(AnalyticsEvents::Ts)
                            .timestamp_with_time_zone()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(AnalyticsEvents::PlayerId).uuid().null())
                    .col(ColumnDef::new(AnalyticsEvents::SessionId).uuid().null())
                    .col(ColumnDef::new(AnalyticsEvents::Kind).string().not_null())
                    .col(
                        ColumnDef::new(AnalyticsEvents::PayloadJson)
                            .json_binary()
                            .null(),
                    )
                    .to_owned(),
            )
            .await?;
        // analytics_rollups
        manager
            .create_table(
                Table::create()
                    .table(AnalyticsRollups::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(AnalyticsRollups::BucketStart)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(ColumnDef::new(AnalyticsRollups::Kind).string().not_null())
                    .col(ColumnDef::new(AnalyticsRollups::Value).double().not_null())
                    .primary_key(
                        Index::create()
                            .col(AnalyticsRollups::BucketStart)
                            .col(AnalyticsRollups::Kind),
                    )
                    .to_owned(),
            )
            .await?;
        // mail_outbox
        manager
            .create_table(
                Table::create()
                    .table(MailOutbox::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(MailOutbox::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(MailOutbox::Recipient).string().not_null())
                    .col(ColumnDef::new(MailOutbox::Subject).string().not_null())
                    .col(ColumnDef::new(MailOutbox::Body).text().not_null())
                    .col(
                        ColumnDef::new(MailOutbox::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::cust("NOW()")),
                    )
                    .to_owned(),
            )
            .await?;
        // jobs
        manager
            .create_table(
                Table::create()
                    .table(Jobs::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Jobs::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Jobs::Kind).string().not_null())
                    .col(ColumnDef::new(Jobs::Payload).text().not_null())
                    .col(
                        ColumnDef::new(Jobs::RunAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::cust("NOW()")),
                    )
                    .col(
                        ColumnDef::new(Jobs::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::cust("NOW()")),
                    )
                    .to_owned(),
            )
            .await?;
        // nodes
        manager
            .create_table(
                Table::create()
                    .table(Nodes::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Nodes::Id).uuid().not_null().primary_key())
                    .col(ColumnDef::new(Nodes::Region).string().not_null())
                    .col(
                        ColumnDef::new(Nodes::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::cust("NOW()")),
                    )
                    .to_owned(),
            )
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_table(Table::drop().table(Nodes::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(Jobs::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(MailOutbox::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(AnalyticsRollups::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(AnalyticsEvents::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(Levels::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(Purchases::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(Entitlements::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(Scores::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(Runs::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(Leaderboards::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(LoginTokens::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(Players::Table).to_owned()).await?;
        Ok(())
    }
}

#[derive(Iden)]
enum Players { Id, Handle, Region, CreatedAt, Table }

#[derive(Iden)]
enum LoginTokens { Token, Player, CreatedAt, Table }

#[derive(Iden)]
enum Leaderboards { Id, Table }

#[derive(Iden)]
enum Runs { Id, Leaderboard, Player, ReplayPath, CreatedAt, Flagged, ReplayIndex, Table }

#[derive(Iden)]
enum Scores { Id, Run, Leaderboard, Player, Points, CreatedAt, Verified, Table }

#[derive(Iden)]
enum Entitlements { Player, Sku, GrantedAt, Table }

#[derive(Iden)]
enum Purchases { Id, Player, Sku, CreatedAt, Table }

#[derive(Iden)]
enum Levels { Id, Name, Data, Table }

#[derive(Iden)]
enum AnalyticsEvents { Ts, PlayerId, SessionId, Kind, PayloadJson, Table }

#[derive(Iden)]
enum AnalyticsRollups { BucketStart, Kind, Value, Table }

#[derive(Iden)]
enum MailOutbox { Id, Recipient, Subject, Body, CreatedAt, Table }

#[derive(Iden)]
enum Jobs { Id, Kind, Payload, RunAt, CreatedAt, Table }

#[derive(Iden)]
enum Nodes { Id, Region, CreatedAt, Table }

