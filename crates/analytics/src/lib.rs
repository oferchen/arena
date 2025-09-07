use std::sync::{Arc, Mutex};

#[cfg(feature = "otlp")]
use opentelemetry::{global, metrics::Counter, KeyValue};
#[cfg(feature = "otlp")]
use std::sync::atomic::{AtomicU64, Ordering};
#[cfg(feature = "prometheus")]
use prometheus::{opts, IntCounterVec};
#[cfg(feature = "posthog")]
use reqwest::Client;
use serde::Serialize;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub enum Event {
    WsConnected,
    MailTestQueued,
    PurchaseInitiated,
    PurchaseCompleted,
    EntitlementChecked,
    RunVerificationFailed,

    // Gameplay
    PlayerJoined,
    PlayerJumped,
    PlayerDied,
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

impl Event {
    pub fn name(&self) -> &'static str {
        match self {
            Event::WsConnected => "ws_connected",
            Event::MailTestQueued => "mail_test_queued",
            Event::PurchaseInitiated => "purchase_initiated",
            Event::PurchaseCompleted => "purchase_completed",
            Event::EntitlementChecked => "entitlement_checked",
            Event::RunVerificationFailed => "run_verification_failed",
            Event::PlayerJoined => "player_joined",
            Event::PlayerJumped => "player_jumped",
            Event::PlayerDied => "player_died",
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
    store: Arc<Mutex<Vec<Event>>>,
    #[cfg(feature = "prometheus")]
    counter: IntCounterVec,
    #[cfg(feature = "posthog")]
    posthog: Option<(Client, String, String)>,
    #[cfg(feature = "otlp")]
    otel: Option<(Counter<u64>, Arc<AtomicU64>)>,
}

impl Analytics {
    pub fn new(posthog_key: Option<String>, enable_otel: bool) -> Self {
        let store = Arc::new(Mutex::new(Vec::new()));

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
            store,
            #[cfg(feature = "prometheus")]
            counter,
            #[cfg(feature = "posthog")]
            posthog,
            #[cfg(feature = "otlp")]
            otel,
        }
    }

    pub fn dispatch(&self, event: Event) {
        self.store.lock().unwrap().push(event.clone());
        let name = event.name();

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
        self.store.lock().unwrap().clone()
    }

    #[cfg(feature = "prometheus")]
    pub fn counter_value(&self, name: &str) -> u64 {
        self.counter.with_label_values(&[name]).get()
    }

    #[cfg(feature = "otlp")]
    pub fn otlp_count(&self) -> u64 {
        self
            .otel
            .as_ref()
            .map(|(_, c)| c.load(Ordering::Relaxed))
            .unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn store_and_prometheus() {
        let analytics = Analytics::new(None, false);
        analytics.dispatch(Event::PlayerJoined);
        assert_eq!(analytics.events(), vec![Event::PlayerJoined]);
        assert_eq!(analytics.counter_value("player_joined"), 1);
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

        let analytics = Analytics::new(Some("test_key".into()), false);
        analytics.dispatch(Event::PlayerJoined);

        tokio::time::sleep(Duration::from_millis(50)).await;
        mock.assert();
    }

    #[cfg(feature = "otlp")]
    #[test]
    fn otlp_counter() {
        let analytics = Analytics::new(None, true);
        analytics.dispatch(Event::PlayerJoined);
        assert_eq!(analytics.otlp_count(), 1);
    }
}

