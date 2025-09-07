use opentelemetry::{KeyValue, global, metrics::Counter};
use prometheus::{IntCounterVec, opts};
use reqwest::Client;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub enum Event {
    WsConnected,
    MailTestQueued,
}

impl Event {
    pub fn name(&self) -> &'static str {
        match self {
            Event::WsConnected => "ws_connected",
            Event::MailTestQueued => "mail_test_queued",
        }
    }
}

#[derive(Clone)]
pub struct Analytics {
    counter: IntCounterVec,
    posthog: Option<(Client, String)>,
    otel: Option<Counter<u64>>,
}

impl Analytics {
    pub fn new(posthog_key: Option<String>, enable_otel: bool) -> Self {
        let counter = IntCounterVec::new(
            opts!("analytics_events_total", "count of analytics events"),
            &["event"],
        )
        .expect("metric can be created");
        let _ = prometheus::default_registry().register(Box::new(counter.clone()));

        let posthog = posthog_key.map(|key| (Client::new(), key));

        let otel = if enable_otel {
            let meter = global::meter("analytics");
            Some(meter.u64_counter("analytics_events").init())
        } else {
            None
        };

        Self {
            counter,
            posthog,
            otel,
        }
    }

    pub fn dispatch(&self, event: Event) {
        let name = event.name();
        self.counter.with_label_values(&[name]).inc();

        if let Some((client, key)) = &self.posthog {
            let payload = serde_json::json!({
                "api_key": key,
                "event": name,
                "distinct_id": "server"
            });
            let client = client.clone();
            tokio::spawn(async move {
                let _ = client
                    .post("https://app.posthog.com/capture/")
                    .json(&payload)
                    .send()
                    .await;
            });
        }

        if let Some(counter) = &self.otel {
            counter.add(1, &[KeyValue::new("event", name)]);
        }
    }
}
