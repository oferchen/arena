use std::sync::Arc;
use std::sync::RwLock;

use ::payments::{Entitlement, UserId};
use chrono::Utc;
use scylla::{IntoTypedRows, Session};

#[derive(Clone, Default)]
pub struct EntitlementStore {
    db: Option<Arc<Session>>,
    inner: Arc<RwLock<Vec<Entitlement>>>,
}

impl EntitlementStore {
    pub fn new(db: Option<Arc<Session>>) -> Self {
        Self {
            db,
            inner: Arc::new(RwLock::new(Vec::new())),
        }
    }

    #[cfg(test)]
    fn inner(&self) -> Arc<RwLock<Vec<Entitlement>>> {
        self.inner.clone()
    }

    pub async fn grant(&self, user_id: UserId, sku_id: String) {
        if let Some(db) = &self.db {
            let query =
                "INSERT INTO entitlements_by_user (user_id, sku_id, granted_at) VALUES (?, ?, ?)";
            let _ = db.query(query, (user_id, sku_id.clone(), Utc::now())).await;
        }
        let mut inner = match self.inner.write() {
            Ok(inner) => inner,
            Err(e) => e.into_inner(),
        };
        if inner
            .iter()
            .any(|e| e.user_id == user_id && e.sku_id == sku_id)
        {
            return;
        }
        inner.push(Entitlement {
            user_id,
            sku_id,
            granted_at: Utc::now(),
        });
    }

    pub async fn list(&self, user_id: &str) -> Vec<String> {
        if let Some(db) = &self.db {
            if let Ok(id) = UserId::parse_str(user_id) {
                let query = "SELECT sku_id FROM entitlements_by_user WHERE user_id = ?";
                if let Ok(res) = db.query(query, (id,)).await {
                    if let Some(rows) = res.rows {
                        return rows
                            .into_typed::<(String,)>()
                            .filter_map(|r| r.ok().map(|(sku,)| sku))
                            .collect();
                    }
                }
            }
        }
        let inner = match self.inner.read() {
            Ok(inner) => inner,
            Err(e) => e.into_inner(),
        };
        inner
            .iter()
            .filter(|e| e.user_id.to_string() == user_id)
            .map(|e| e.sku_id.clone())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn grant_recovers_from_poison() {
        let store = EntitlementStore::new(None);

        let inner = store.inner();
        let inner_clone = inner.clone();
        let _ = std::thread::spawn(move || {
            let _guard = inner_clone.write().unwrap();
            panic!("poison");
        })
        .join();

        let user = UserId::new_v4();
        store.grant(user, "sku".to_string()).await;
        let list = store.list(&user.to_string()).await;
        assert_eq!(list, vec!["sku".to_string()]);
    }

    #[tokio::test]
    async fn list_recovers_from_poison() {
        let store = EntitlementStore::new(None);
        let user = UserId::new_v4();
        store.grant(user, "sku".to_string()).await;

        let inner = store.inner();
        let inner_clone = inner.clone();
        let _ = std::thread::spawn(move || {
            let _guard = inner_clone.write().unwrap();
            panic!("poison");
        })
        .join();

        let list = store.list(&user.to_string()).await;
        assert_eq!(list, vec!["sku".to_string()]);
    }
}
