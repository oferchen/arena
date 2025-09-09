use std::time::Duration;

use chrono::{Duration as ChronoDuration, Utc};
use sea_orm::{ActiveValue::Set, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, QuerySelect, TransactionTrait, DbErr};
use sea_orm::sea_query::{OnConflict, LockType, LockBehavior};
use uuid::Uuid;

use crate::entities::{jobs, nodes};

const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);
const LEADER_TIMEOUT: ChronoDuration = ChronoDuration::seconds(15);

/// Run the background job runner.
pub async fn run(db: DatabaseConnection) {
    let node_id = Uuid::new_v4();
    let region = std::env::var("ARENA_REGION").unwrap_or_else(|_| "global".to_string());
    let mut interval = tokio::time::interval(HEARTBEAT_INTERVAL);
    loop {
        interval.tick().await;
        if let Err(e) = heartbeat(&db, node_id, &region).await {
            log::error!("heartbeat failed: {e}");
            continue;
        }
        match is_leader(&db, node_id).await {
            Ok(true) => {
                if let Err(e) = claim_and_run(&db).await {
                    log::error!("job runner error: {e}");
                }
            }
            Ok(false) => {}
            Err(e) => log::error!("leader check failed: {e}"),
        }
    }
}

async fn heartbeat(db: &DatabaseConnection, id: Uuid, region: &str) -> Result<(), DbErr> {
    let model = nodes::ActiveModel {
        id: Set(id),
        region: Set(region.to_owned()),
        created_at: Set(Utc::now()),
    };
    nodes::Entity::insert(model)
        .on_conflict(
            OnConflict::column(nodes::Column::Id)
                .update_columns([nodes::Column::Region, nodes::Column::CreatedAt])
                .to_owned(),
        )
        .exec(db)
        .await?;
    Ok(())
}

async fn is_leader(db: &DatabaseConnection, id: Uuid) -> Result<bool, DbErr> {
    let cutoff = Utc::now() - LEADER_TIMEOUT;
    let leader = nodes::Entity::find()
        .filter(nodes::Column::CreatedAt.gt(cutoff))
        .order_by_asc(nodes::Column::Id)
        .one(db)
        .await?;
    Ok(matches!(leader, Some(n) if n.id == id))
}

async fn claim_and_run(db: &DatabaseConnection) -> Result<(), DbErr> {
    let now = Utc::now();
    let txn = db.begin().await?;
    if let Some(job) = jobs::Entity::find()
        .filter(jobs::Column::RunAt.lte(now))
        .order_by_asc(jobs::Column::RunAt)
        .lock_with_behavior(LockType::Update, LockBehavior::SkipLocked)
        .one(&txn)
        .await?
    {
        jobs::Entity::delete_by_id(job.id).exec(&txn).await?;
        txn.commit().await?;
        handle(job).await;
    } else {
        txn.commit().await?;
    }
    Ok(())
}

async fn handle(job: jobs::Model) {
    log::info!("ran job {}", job.id);
}

