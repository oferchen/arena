use std::sync::Arc;
use std::sync::RwLock;

use ::payments::{Entitlement, UserId};
use chrono::Utc;
use sea_orm::{DatabaseConnection, DbBackend, Statement, TryGetable};

#[derive(Clone, Default)]
pub struct EntitlementStore {
    db: Option<DatabaseConnection>,
    inner: Arc<RwLock<Vec<Entitlement>>>,
}

impl EntitlementStore {
    pub fn new(db: Option<DatabaseConnection>) -> Self {
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
            let stmt = Statement::from_sql_and_values(
                DbBackend::Postgres,
                "INSERT INTO entitlements_by_user (user_id, sku_id, granted_at) VALUES ($1, $2, NOW())",
                vec![user_id.to_string().into(), sku_id.clone().into()],
            );
            let _ = db.execute(stmt).await;
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
                let stmt = Statement::from_sql_and_values(
                    DbBackend::Postgres,
                    "SELECT sku_id FROM entitlements_by_user WHERE user_id = $1",
                    vec![id.to_string().into()],
                );
                if let Ok(rows) = db.query_all(stmt).await {
                    return rows
                        .into_iter()
                        .filter_map(|row| row.try_get::<String>("sku_id").ok())
                        .collect();
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
