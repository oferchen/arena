use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::RwLock;
pub use uuid::Uuid as UserId;

#[derive(Clone, Serialize, Deserialize)]
pub struct Sku {
    pub id: String,
    pub price_cents: u32,
}

#[derive(Clone)]
pub struct Catalog {
    skus: Vec<Sku>,
}

impl Catalog {
    pub fn new(skus: Vec<Sku>) -> Self {
        Self { skus }
    }

    pub fn get(&self, id: &str) -> Option<&Sku> {
        self.skus.iter().find(|s| s.id == id)
    }
}

pub struct CheckoutSession {
    pub url: String,
}

#[derive(Clone, Default)]
pub struct StripeClient;

impl StripeClient {
    pub fn new() -> Self {
        Self
    }

    pub async fn create_checkout_session(&self, sku: &Sku) -> CheckoutSession {
        CheckoutSession { url: format!("https://payments.example/checkout/{}", sku.id) }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Entitlement {
    pub user_id: UserId,
    pub sku_id: String,
    pub granted_at: DateTime<Utc>,
}

#[derive(Default)]
pub struct EntitlementStore {
    inner: RwLock<Vec<Entitlement>>,
}

impl Clone for EntitlementStore {
    fn clone(&self) -> Self {
        let data = self.inner.read().unwrap().clone();
        Self { inner: RwLock::new(data) }
    }
}

impl EntitlementStore {
    pub fn grant(&self, user_id: UserId, sku_id: String) {
        let ent = Entitlement { user_id, sku_id, granted_at: Utc::now() };
        self.inner.write().unwrap().push(ent);
    }

    pub fn has(&self, user_id: UserId, sku_id: &str) -> bool {
        self.inner
            .read()
            .unwrap()
            .iter()
            .any(|e| e.user_id == user_id && e.sku_id == sku_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grants_entitlement() {
        let store = EntitlementStore::default();
        let user = UserId::new_v4();
        store.grant(user, "pro".to_string());
        assert!(store.has(user, "pro"));
    }
=======
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct Sku {
    pub id: &'static str,
    pub name: &'static str,
    pub price_cents: u32,
}

static CATALOG: &[Sku] = &[
    Sku { id: "duck_hunt", name: "Duck Hunt Module", price_cents: 199 },
];

pub fn catalog() -> &'static [Sku] {
    CATALOG
}

#[derive(Clone, Default)]
pub struct EntitlementStore {
    inner: Arc<Mutex<HashMap<String, HashSet<String>>>>,
}

impl EntitlementStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn load(path: &Path) -> Self {
        if let Ok(data) = std::fs::read_to_string(path) {
            if let Ok(map) = serde_json::from_str::<HashMap<String, HashSet<String>>>(&data) {
                return Self { inner: Arc::new(Mutex::new(map)) };
            }
        }
        Self::new()
    }

    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        let data = self.inner.lock().unwrap();
        let json = serde_json::to_string(&*data).unwrap();
        std::fs::write(path, json)
    }

    pub fn grant(&self, user: &str, sku: &str) {
        let mut map = self.inner.lock().unwrap();
        map.entry(user.to_string())
            .or_insert_with(HashSet::new)
            .insert(sku.to_string());
    }

    pub fn has(&self, user: &str, sku: &str) -> bool {
        self.inner
            .lock()
            .unwrap()
            .get(user)
            .map_or(false, |set| set.contains(sku))
    }

    pub fn list(&self, user: &str) -> Vec<String> {
        self.inner
            .lock()
            .unwrap()
            .get(user)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .collect()
    }
}

#[derive(Serialize, Deserialize)]
pub struct EntitlementList {
    pub entitlements: Vec<String>,
}

pub fn initiate_purchase(_user: &str, sku: &str) -> String {
    format!("session_{sku}")
}

pub fn complete_purchase(store: &EntitlementStore, user: &str, sku: &str) {
    store.grant(user, sku);
}
