pub mod db;
pub mod models;

use std::io;
use std::path::PathBuf;

use anyhow::Result;
use chrono::{Duration, Utc};
use db::{purchases, runs, scores};
use models::{LeaderboardWindow, Run, Score};
use sea_orm::sea_query::TableCreateStatement;
use sea_orm::{
    ActiveModelTrait,
    ActiveValue::Set,
    ColumnTrait,
    ConnectionTrait,
    Database,
    DatabaseConnection,
    EntityTrait,
    JoinType,
    QueryFilter,
    QueryOrder,
    QuerySelect,
    RelationTrait,
    Schema,
};
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use uuid::Uuid;

const WINDOWS: [LeaderboardWindow; 3] = [
    LeaderboardWindow::Daily,
    LeaderboardWindow::Weekly,
    LeaderboardWindow::AllTime,
];

#[derive(Clone)]
pub struct LeaderboardService {
    db: DatabaseConnection,
    replay_dir: PathBuf,
    tx: broadcast::Sender<LeaderboardSnapshot>,
    max: usize,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct LeaderboardSnapshot {
    pub leaderboard: Uuid,
    pub window: LeaderboardWindow,
    pub scores: Vec<Score>,
}

impl LeaderboardService {
    pub async fn new(database_url: &str, replay_dir: PathBuf) -> Result<Self> {
        let db = Database::connect(database_url).await?;
        let schema = Schema::new(db.get_database_backend());

        create_table(
            &db,
            schema
                .create_table_from_entity(runs::Entity)
                .if_not_exists()
                .to_owned(),
        )
        .await?;
        create_table(
            &db,
            schema
                .create_table_from_entity(scores::Entity)
                .if_not_exists()
                .to_owned(),
        )
        .await?;
        create_table(
            &db,
            schema
                .create_table_from_entity(purchases::Entity)
                .if_not_exists()
                .to_owned(),
        )
        .await?;

        tokio::fs::create_dir_all(&replay_dir).await?;
        let (tx, _) = broadcast::channel(16);
        let max = std::env::var("ARENA_LEADERBOARD_MAX")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(100);
        Ok(Self {
            db,
            replay_dir,
            tx,
            max,
        })
    }

    pub async fn submit_score(
        &self,
        leaderboard: Uuid,
        score: Score,
        mut run: Run,
        replay: Vec<u8>,
    ) -> io::Result<()> {
        if !replay.is_empty() {
            let filename = format!("{}", run.id);
            let path = self.replay_dir.join(&filename);
            tokio::fs::write(&path, &replay).await?;
            run.replay_path = filename;
        }

        let run_model = runs::ActiveModel {
            id: Set(run.id),
            leaderboard_id: Set(leaderboard),
            player_id: Set(run.player_id),
            replay_path: Set(run.replay_path.clone()),
            created_at: Set(run.created_at),
            flagged: Set(run.flagged),
            replay_index: Set(run.replay_index),
        };
        run_model.insert(&self.db).await.map_err(to_io_error)?;

        let score_model = scores::ActiveModel {
            id: Set(score.id),
            run_id: Set(run.id),
            leaderboard_id: Set(leaderboard),
            player_id: Set(score.player_id),
            points: Set(score.points),
            created_at: Set(score.created_at),
            verified: Set(score.verified),
        };
        score_model.insert(&self.db).await.map_err(to_io_error)?;

        for window in WINDOWS {
            let scores = self.get_scores(leaderboard, window).await;
            let _ = self.tx.send(LeaderboardSnapshot {
                leaderboard,
                window,
                scores,
            });
        }
        Ok(())
    }

    pub async fn get_scores(
        &self,
        leaderboard: Uuid,
        window: LeaderboardWindow,
    ) -> Vec<Score> {
        let now = Utc::now();
        let mut query = scores::Entity::find()
            .filter(scores::Column::LeaderboardId.eq(leaderboard))
            .join(JoinType::InnerJoin, scores::Relation::Runs.def())
            .filter(runs::Column::Flagged.eq(false))
            .order_by_desc(scores::Column::Points)
            .limit(self.max as u64);

        match window {
            LeaderboardWindow::Daily => {
                query = query.filter(scores::Column::CreatedAt.gte(now - Duration::days(1)));
            }
            LeaderboardWindow::Weekly => {
                query = query.filter(scores::Column::CreatedAt.gte(now - Duration::weeks(1)));
            }
            LeaderboardWindow::AllTime => {}
        }

        query
            .all(&self.db)
            .await
            .unwrap_or_default()
            .into_iter()
            .map(|s| Score {
                id: s.id,
                run_id: s.run_id,
                player_id: s.player_id,
                points: s.points,
                verified: s.verified,
                created_at: s.created_at,
                window,
            })
            .collect()
    }

    pub async fn record_purchase(&self, user_id: Uuid, sku: &str) -> Result<Uuid> {
        let id = Uuid::new_v4();
        let purchase = purchases::ActiveModel {
            id: Set(id),
            user_id: Set(user_id),
            sku: Set(sku.to_string()),
            created_at: Set(Utc::now()),
        };
        purchase.insert(&self.db).await?;
        Ok(id)
    }

    pub fn subscribe(&self) -> broadcast::Receiver<LeaderboardSnapshot> {
        self.tx.subscribe()
    }

    pub async fn get_replay(&self, run_id: Uuid) -> Option<Vec<u8>> {
        if let Ok(Some(run)) = runs::Entity::find_by_id(run_id).one(&self.db).await {
            let path = self.replay_dir.join(run.replay_path);
            return tokio::fs::read(path).await.ok();
        }
        None
    }

    pub async fn verify_run(&self, _run_id: Uuid) -> bool {
        // Updating verification status is left as future work.
        false
    }
}

async fn create_table(db: &DatabaseConnection, stmt: TableCreateStatement) -> Result<()> {
    let builder = db.get_database_backend();
    db.execute(builder.build(&stmt)).await?;
    Ok(())
}

fn to_io_error<E: std::error::Error + Send + Sync + 'static>(e: E) -> io::Error {
    io::Error::new(io::ErrorKind::Other, e)
}
