use std::time::Duration;

use chrono::{Duration as ChronoDuration, Utc};
use sea_orm::sea_query::{LockBehavior, LockType, OnConflict};
use sea_orm::{
    ActiveValue::Set, ColumnTrait, DatabaseConnection, DbErr, EntityTrait, QueryFilter, QueryOrder,
    QuerySelect, TransactionTrait,
};
use serde_json::json;
use uuid::Uuid;

use crate::entities::{jobs, nodes};

const MAX_ATTEMPTS: i32 = 5;

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
        last_seen: Set(Utc::now()),
        info: Set(json!({})),
    };
    nodes::Entity::insert(model)
        .on_conflict(
            OnConflict::column(nodes::Column::Id)
                .update_columns([nodes::Column::Region, nodes::Column::LastSeen, nodes::Column::Info])
                .to_owned(),
        )
        .exec(db)
        .await?;
    Ok(())
}

async fn is_leader(db: &DatabaseConnection, id: Uuid) -> Result<bool, DbErr> {
    let cutoff = Utc::now() - LEADER_TIMEOUT;
    let leader = nodes::Entity::find()
        .filter(nodes::Column::LastSeen.gt(cutoff))
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
        let mut active: jobs::ActiveModel = job.into();
        active.status = Set(jobs::JobStatus::Running);
        active.attempts = Set(active.attempts.unwrap_or_default() + 1);
        active.updated_at = Set(now);
        let job = jobs::Entity::update(active).exec(&txn).await?;
        txn.commit().await?;

        let res = handle(job.clone()).await;
        let mut active: jobs::ActiveModel = job.into();
        active.updated_at = Set(Utc::now());
        match res {
            Ok(_) => {
                active.status = Set(jobs::JobStatus::Done);
            }
            Err(e) => {
                let job_id = active.id.clone().unwrap();
                log::error!("job {} failed: {e}", job_id);
                let attempts = active.attempts.unwrap_or_default();
                if attempts >= MAX_ATTEMPTS {
                    active.status = Set(jobs::JobStatus::Failed);
                } else {
                    active.status = Set(jobs::JobStatus::Pending);
                    active.run_at = Set(Utc::now() + ChronoDuration::seconds(60));
                }
            }
        }
        jobs::Entity::update(active).exec(db).await?;
    } else {
        txn.commit().await?;
    }
    Ok(())
}

async fn handle(job: jobs::Model) -> anyhow::Result<()> {
    match job.kind.as_str() {
        "fail" => anyhow::bail!("intentional failure"),
        _ => {
            log::info!("ran job {}", job.id);
            Ok(())
        }
    }
}
