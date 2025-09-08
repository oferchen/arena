use std::sync::{Arc, Mutex};

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
const MAX_EVENTS_ENV_VAR: &str = "ANALYTICS_MAX_EVENTS";

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
    names: Vec<&'static str>,
    data: Vec<Option<String>>,
    events: Vec<Event>,
    max_len: usize,
}

impl ColumnarStore {
    fn new(max_len: usize) -> Self {
        Self {
            names: Vec::new(),
            data: Vec::new(),
            events: Vec::new(),
            max_len,
        }
    }

    fn push(&mut self, name: &'static str, data: Option<String>, event: Event) {
        if self.events.len() >= self.max_len {
            self.names.remove(0);
            self.data.remove(0);
            self.events.remove(0);
        }
        self.names.push(name);
        self.data.push(data);
        self.events.push(event);
    }

    fn events(&self) -> Vec<Event> {
        self.events.clone()
    }

    fn take_events(&mut self) -> Vec<Event> {
        let events = self.events.clone();
        self.names.clear();
        self.data.clear();
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

#[derive(Clone)]
pub struct Analytics {
    enabled: bool,
    store: Arc<Mutex<ColumnarStore>>,
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
        posthog_key: Option<String>,
        enable_otel: bool,
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
        let otel = if enable_otel {
            let meter = global::meter("analytics");
            let counter = meter.u64_counter("analytics_events").init();
            let calls = Arc::new(AtomicU64::new(0));
            Some((counter, calls))
        } else {
            None
        };
        #[cfg(not(feature = "otlp"))]
        let _ = enable_otel;

        Self {
            enabled,
            store,
            #[cfg(feature = "prometheus")]
            counter,
            #[cfg(feature = "posthog")]
            posthog,
            #[cfg(feature = "otlp")]
            otel,
        }
    }

    pub fn new(enabled: bool, posthog_key: Option<String>, enable_otel: bool) -> Self {
        let max_events = std::env::var(MAX_EVENTS_ENV_VAR)
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(DEFAULT_MAX_EVENTS);
        Self::with_max_events(enabled, posthog_key, enable_otel, max_events)
    }

    pub fn dispatch(&self, event: Event) {
        if !self.enabled {
            return;
        }
        let name = event.name();
        let data = match &event {
            Event::Error { message } => Some(message.clone()),
            _ => None,
        };
        self.store.lock().unwrap().push(name, data, event.clone());

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

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "prometheus")]
    #[test]
    fn store_and_prometheus() {
        let analytics = Analytics::new(true, None, false);
        analytics.dispatch(Event::ShotFired);
        assert_eq!(analytics.events(), vec![Event::ShotFired]);
        assert_eq!(analytics.counter_value("shot_fired"), 1);
    }

    #[cfg(not(feature = "prometheus"))]
    #[test]
    fn store() {
        let analytics = Analytics::new(true, None, false);
        analytics.dispatch(Event::ShotFired);
        assert_eq!(analytics.events(), vec![Event::ShotFired]);
    }

    #[test]
    fn ring_buffer_limit() {
        unsafe { std::env::set_var(MAX_EVENTS_ENV_VAR, "2") };
        let analytics = Analytics::new(true, None, false);
        analytics.dispatch(Event::ShotFired);
        analytics.dispatch(Event::TargetHit);
        analytics.dispatch(Event::Death);
        assert_eq!(analytics.events(), vec![Event::TargetHit, Event::Death]);
        unsafe { std::env::remove_var(MAX_EVENTS_ENV_VAR) };
    }

    #[test]
    fn flush_clears_events() {
        let analytics = Analytics::with_max_events(true, None, false, 2);
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

        unsafe {
            std::env::set_var("POSTHOG_ENDPOINT", server.url("/capture/"));
        }

        let analytics = Analytics::new(true, Some("test_key".into()), false);
        analytics.dispatch(Event::ShotFired);

        tokio::time::sleep(Duration::from_millis(50)).await;
        mock.assert();
    }

    #[cfg(feature = "otlp")]
    #[test]
    fn otlp_counter() {
        let analytics = Analytics::new(true, None, true);
        analytics.dispatch(Event::ShotFired);
        assert_eq!(analytics.otlp_count(), 1);
    }
}
