//! Utilities for collecting analytics events for Arena.
//!
//! Up to `DEFAULT_MAX_EVENTS` events are retained in memory. Set the
//! `ARENA_ANALYTICS_MAX_EVENTS` environment variable to change this limit.

use std::{
    net::SocketAddr,
    sync::{Arc, Mutex},
};

use chrono::Utc;
use sea_orm::{
    DatabaseConnection, DbBackend, Set, Statement,
    entity::prelude::*,
    sea_query::{Alias, Expr, Func, OnConflict, PostgresQueryBuilder, Query, SimpleExpr},
};
use serde_json::{Value as JsonValue, json};
use tokio::time::{Duration, interval};
use uuid::Uuid;

#[cfg(feature = "bevy-resource")]
use bevy_ecs::system::Resource;
#[cfg(feature = "otlp")]
use opentelemetry::{KeyValue, global, metrics::Counter};
#[cfg(feature = "prometheus")]
use prometheus::{IntCounterVec, opts};
#[cfg(feature = "posthog")]
use reqwest::Client;
use serde::Serialize;
#[cfg(feature = "otlp")]
use std::sync::atomic::{AtomicU64, Ordering};

const DEFAULT_MAX_EVENTS: usize = 10_000;
const MAX_EVENTS_ENV_VAR: &str = "ARENA_ANALYTICS_MAX_EVENTS";

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub enum Event {
    WsConnected,
    MailTestQueued,
    PurchaseCompleted { sku: String, user: String },
    EntitlementChecked,
    RunVerificationFailed,

    SessionStart,
    LevelStart { level: u32 },
    StoreOpen,
    Error { message: String },

    // Gameplay
    PlayerJoined,
    PlayerJumped,
    PlayerDied,
    ShotFired,
    TargetHit,
    DamageTaken,
    Death,
    Respawn,
    LeaderboardSubmit,
    // Economy
    ItemPurchased,
    CurrencyEarned,
    CurrencySpent,
    // Performance
    FrameDropped,
    HighLatency,
    TickOverrun,

    StoreViewed,
    PurchaseInitiated,
    PurchaseSucceeded,
    EntitlementGranted,
}

struct ColumnarStore {
    events: Vec<Event>,
    max_len: usize,
}

impl ColumnarStore {
    fn new(max_len: usize) -> Self {
        Self {
            events: Vec::new(),
            max_len,
        }
    }

    fn push(&mut self, event: Event) {
        if self.events.len() >= self.max_len {
            self.events.remove(0);
        }
        self.events.push(event);
    }

    fn events(&self) -> Vec<Event> {
        self.events.clone()
    }

    fn take_events(&mut self) -> Vec<Event> {
        let events = self.events.clone();
        self.events.clear();
        events
    }
}

impl Default for ColumnarStore {
    fn default() -> Self {
        Self::new(DEFAULT_MAX_EVENTS)
    }
}
impl Event {
    pub fn name(&self) -> &'static str {
        match self {
            Event::WsConnected => "ws_connected",
            Event::MailTestQueued => "mail_test_queued",
            Event::PurchaseCompleted { .. } => "purchase_completed",
            Event::EntitlementChecked => "entitlement_checked",
            Event::RunVerificationFailed => "run_verification_failed",
            Event::SessionStart => "session_start",
            Event::LevelStart { .. } => "level_start",
            Event::StoreOpen => "store_open",
            Event::Error { .. } => "error",
            Event::PlayerJoined => "player_joined",
            Event::PlayerJumped => "player_jumped",
            Event::PlayerDied => "player_died",
            Event::ShotFired => "shot_fired",
            Event::TargetHit => "target_hit",
            Event::DamageTaken => "damage_taken",
            Event::Death => "death",
            Event::Respawn => "respawn",
            Event::LeaderboardSubmit => "leaderboard_submit",
            Event::ItemPurchased => "item_purchased",
            Event::CurrencyEarned => "currency_earned",
            Event::CurrencySpent => "currency_spent",
            Event::FrameDropped => "frame_dropped",
            Event::HighLatency => "high_latency",
            Event::TickOverrun => "tick_overrun",
            Event::StoreViewed => "store_viewed",
            Event::PurchaseInitiated => "purchase_initiated",
            Event::PurchaseSucceeded => "purchase_succeeded",
            Event::EntitlementGranted => "entitlement_granted",
        }
    }
}

#[cfg_attr(feature = "bevy-resource", derive(Resource))]
#[derive(Clone)]
pub struct Analytics {
    enabled: bool,
    store: Arc<Mutex<ColumnarStore>>,
    db: Option<DatabaseConnection>,
    #[cfg(feature = "prometheus")]
    counter: IntCounterVec,
    #[cfg(feature = "posthog")]
    posthog: Option<(Client, String, String)>,
    #[cfg(feature = "otlp")]
    otel: Option<(Counter<u64>, Arc<AtomicU64>)>,
}

impl Analytics {
    pub fn with_max_events(
        enabled: bool,
        db: Option<DatabaseConnection>,
        posthog_key: Option<String>,
        metrics_addr: Option<SocketAddr>,
        max_events: usize,
    ) -> Self {
        let store = Arc::new(Mutex::new(ColumnarStore::new(max_events)));

        #[cfg(feature = "prometheus")]
        let counter = {
            let c = IntCounterVec::new(
                opts!("analytics_events_total", "count of analytics events"),
                &["event"],
            )
            .expect("metric can be created");
            let _ = prometheus::default_registry().register(Box::new(c.clone()));
            c
        };

        #[cfg(feature = "posthog")]
        let posthog = posthog_key.map(|key| {
            let endpoint = std::env::var("POSTHOG_ENDPOINT")
                .unwrap_or_else(|_| "https://app.posthog.com/capture/".to_string());
            (Client::new(), key, endpoint)
        });
        #[cfg(not(feature = "posthog"))]
        let _ = posthog_key;

        #[cfg(feature = "otlp")]
        let otel = if metrics_addr.is_some() {
            let meter = global::meter("analytics");
            let counter = meter.u64_counter("analytics_events").init();
            let calls = Arc::new(AtomicU64::new(0));
            Some((counter, calls))
        } else {
            None
        };
        #[cfg(not(feature = "otlp"))]
        let _ = metrics_addr;

        let analytics = Self {
            enabled,
            store,
            db,
            #[cfg(feature = "prometheus")]
            counter,
            #[cfg(feature = "posthog")]
            posthog,
            #[cfg(feature = "otlp")]
            otel,
        };

        if analytics.db.is_some() {
            // periodically flush events to the database
            {
                let this = analytics.clone();
                tokio::spawn(async move {
                    let mut ticker = interval(Duration::from_secs(5));
                    loop {
                        ticker.tick().await;
                        let _ = this.flush_to_db().await;
                    }
                });
            }

            // periodically roll up events
            {
                let this = analytics.clone();
                tokio::spawn(async move {
                    let mut ticker = interval(Duration::from_secs(60 * 60));
                    loop {
                        ticker.tick().await;
                        let _ = this.rollup().await;
                    }
                });
            }
        }

        analytics
    }

    pub fn new(
        enabled: bool,
        db: Option<DatabaseConnection>,
        posthog_key: Option<String>,
        metrics_addr: Option<SocketAddr>,
    ) -> Self {
        let max_events = std::env::var(MAX_EVENTS_ENV_VAR)
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(DEFAULT_MAX_EVENTS);
        Self::with_max_events(enabled, db, posthog_key, metrics_addr, max_events)
    }

    pub fn dispatch(&self, event: Event) {
        if !self.enabled {
            return;
        }
        let name = event.name();
        self.store.lock().unwrap().push(event.clone());

        #[cfg(feature = "prometheus")]
        self.counter.with_label_values(&[name]).inc();

        #[cfg(feature = "posthog")]
        if let Some((client, key, endpoint)) = &self.posthog {
            let payload = serde_json::json!({
                "api_key": key,
                "event": name,
                "distinct_id": "server",
            });
            let client = client.clone();
            let endpoint = endpoint.clone();
            tokio::spawn(async move {
                let _ = client.post(endpoint).json(&payload).send().await;
            });
        }

        #[cfg(feature = "otlp")]
        if let Some((counter, calls)) = &self.otel {
            counter.add(1, &[KeyValue::new("event", name)]);
            calls.fetch_add(1, Ordering::Relaxed);
        }
    }

    async fn flush_to_db(&self) -> Result<(), DbErr> {
        if !self.enabled {
            return Ok(());
        }
        let events = self.store.lock().unwrap().take_events();
        if events.is_empty() {
            return Ok(());
        }
        if let Some(db) = &self.db {
            let mut models = Vec::with_capacity(events.len());
            for event in events {
                let payload = match &event {
                    Event::Error { message } => Some(json!({ "message": message })),
                    Event::PurchaseCompleted { sku, user } => {
                        Some(json!({ "sku": sku, "user": user }))
                    }
                    _ => None,
                };
                models.push(events::ActiveModel {
                    ts: Set(Utc::now()),
                    player_id: Set(None),
                    session_id: Set(None),
                    kind: Set(event.name().to_string()),
                    payload_json: Set(payload),
                    ..Default::default()
                });
            }
            events::Entity::insert_many(models).exec(db).await?;
        }
        Ok(())
    }

    async fn rollup(&self) -> Result<(), DbErr> {
        if !self.enabled {
            return Ok(());
        }
        let db = if let Some(db) = &self.db {
            db
        } else {
            return Ok(());
        };
        let now = Utc::now();
        let from = now - chrono::Duration::hours(1);
        let select = Query::select()
            .expr_as(
                Func::cust(Alias::new("date_trunc"))
                    .arg("hour")
                    .arg(Expr::col(events::Column::Ts)),
                Alias::new("bucket_start"),
            )
            .expr_as(Expr::col(events::Column::Kind), Alias::new("kind"))
            .expr_as(
                Func::count(Expr::col(events::Column::Kind)),
                Alias::new("value"),
            )
            .from(events::Entity)
            .and_where(Expr::col(events::Column::Ts).gte(from))
            .and_where(Expr::col(events::Column::Ts).lt(now))
            .add_group_by([
                Into::<SimpleExpr>::into(
                    Func::cust(Alias::new("date_trunc"))
                        .arg("hour")
                        .arg(Expr::col(events::Column::Ts)),
                ),
                SimpleExpr::from(Expr::col(events::Column::Kind)),
            ])
            .to_owned();
        let insert = Query::insert()
            .into_table(rollups::Entity)
            .columns([
                rollups::Column::BucketStart,
                rollups::Column::Kind,
                rollups::Column::Value,
            ])
            .select_from(select)
            .unwrap()
            .on_conflict(
                OnConflict::columns([rollups::Column::BucketStart, rollups::Column::Kind])
                    .update_column(rollups::Column::Value)
                    .to_owned(),
            )
            .build(PostgresQueryBuilder);
        db.execute(Statement::from_sql_and_values(
            DbBackend::Postgres,
            insert.0,
            insert.1,
        ))
        .await?;
        Ok(())
    }

    pub fn events(&self) -> Vec<Event> {
        self.store.lock().unwrap().events()
    }

    pub fn flush(&self) -> Vec<Event> {
        self.store.lock().unwrap().take_events()
    }

    #[cfg(feature = "prometheus")]
    pub fn counter_value(&self, name: &str) -> u64 {
        self.counter.with_label_values(&[name]).get()
    }

    #[cfg(feature = "otlp")]
    pub fn otlp_count(&self) -> u64 {
        self.otel
            .as_ref()
            .map(|(_, c)| c.load(Ordering::Relaxed))
            .unwrap_or(0)
    }
}

mod events {
    use super::{JsonValue, Uuid};
    use sea_orm::entity::prelude::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "analytics_events")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i64,
        pub ts: DateTimeUtc,
        pub player_id: Option<String>,
        pub session_id: Option<Uuid>,
        pub kind: String,
        pub payload_json: Option<JsonValue>,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

mod rollups {
    use sea_orm::entity::prelude::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "analytics_rollups")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub bucket_start: DateTimeUtc,
        #[sea_orm(primary_key, auto_increment = false)]
        pub kind: String,
        pub value: f64,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsStr;

    // Safe wrappers around environment variable mutation used only in tests.
    fn set_var<K: AsRef<OsStr>, V: AsRef<OsStr>>(key: K, value: V) {
        // SAFETY: tests run in a controlled environment and no other threads
        // modify environment variables concurrently.
        unsafe { std::env::set_var(key, value) }
    }

    fn remove_var<K: AsRef<OsStr>>(key: K) {
        // SAFETY: see `set_var` above.
        unsafe { std::env::remove_var(key) }
    }

    #[cfg(feature = "prometheus")]
    #[test]
    fn store_and_prometheus() {
        let analytics = Analytics::new(true, None, None, None);
        analytics.dispatch(Event::ShotFired);
        assert_eq!(analytics.events(), vec![Event::ShotFired]);
        assert_eq!(analytics.counter_value("shot_fired"), 1);
    }

    #[cfg(not(feature = "prometheus"))]
    #[test]
    fn store() {
        let analytics = Analytics::new(true, None, None, None);
        analytics.dispatch(Event::ShotFired);
        assert_eq!(analytics.events(), vec![Event::ShotFired]);
    }

    #[test]
    fn ring_buffer_limit() {
        set_var(MAX_EVENTS_ENV_VAR, "2");
        let analytics = Analytics::new(true, None, None, None);
        analytics.dispatch(Event::ShotFired);
        analytics.dispatch(Event::TargetHit);
        analytics.dispatch(Event::Death);
        assert_eq!(analytics.events(), vec![Event::TargetHit, Event::Death]);
        remove_var(MAX_EVENTS_ENV_VAR);
    }

    #[test]
    fn flush_clears_events() {
        let analytics = Analytics::with_max_events(true, None, None, None, 2);
        analytics.dispatch(Event::ShotFired);
        let flushed = analytics.flush();
        assert_eq!(flushed, vec![Event::ShotFired]);
        assert!(analytics.events().is_empty());
    }

    #[cfg(feature = "posthog")]
    #[tokio::test]
    async fn posthog_sink() {
        use httpmock::{Method::POST, MockServer};
        use std::time::Duration;

        let server = MockServer::start();
        let mock = server.mock(|when, then| {
            when.method(POST).path("/capture/");
            then.status(200);
        });

        set_var("POSTHOG_ENDPOINT", server.url("/capture/"));

        let analytics = Analytics::new(true, None, Some("test_key".into()), None);
        analytics.dispatch(Event::ShotFired);

        tokio::time::sleep(Duration::from_millis(50)).await;
        mock.assert();
    }

    #[cfg(feature = "otlp")]
    #[test]
    fn otlp_counter() {
        let analytics = Analytics::new(true, None, None, Some("127.0.0.1:0".parse().unwrap()));
        analytics.dispatch(Event::ShotFired);
        assert_eq!(analytics.otlp_count(), 1);
    }
}
