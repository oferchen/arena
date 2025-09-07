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
}
