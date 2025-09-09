use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::RwLock;
pub use uuid::Uuid as UserId;
use sea_orm::{
    entity::prelude::*,
    sea_query::OnConflict,
    ActiveValue::Set,
    DatabaseConnection,
    QueryFilter,
    TransactionTrait,
    TransactionError,
};

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

    pub fn all(&self) -> &[Sku] {
        &self.skus
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
        Self {
            inner: RwLock::new(data),
        }
    }
}

impl EntitlementStore {
    pub fn grant(&self, user_id: UserId, sku_id: String) {
        let mut inner = self.inner.write().unwrap();
        if inner
            .iter()
            .any(|e| e.user_id == user_id && e.sku_id == sku_id)
        {
            return;
        }
        let ent = Entitlement {
            user_id,
            sku_id,
            granted_at: Utc::now(),
        };
        inner.push(ent);
    }

    pub fn has(&self, user_id: UserId, sku_id: &str) -> bool {
        self.inner
            .read()
            .unwrap()
            .iter()
            .any(|e| e.user_id == user_id && e.sku_id == sku_id)
    }

    pub fn list(&self, user_id: &str) -> Vec<String> {
        match UserId::parse_str(user_id) {
            Ok(id) => self
                .inner
                .read()
                .unwrap()
                .iter()
                .filter(|e| e.user_id == id)
                .map(|e| e.sku_id.clone())
                .collect(),
            Err(_) => Vec::new(),
        }
    }

    pub fn save(&self, path: &Path) -> Result<(), StoreError> {
        let data = self.inner.read().unwrap();
        let json = serde_json::to_string(&*data)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    pub fn load(&self, path: &Path) -> Result<(), StoreError> {
        let data = match std::fs::read(path) {
            Ok(data) => data,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(e) => return Err(e.into()),
        };
        let entitlements: Vec<Entitlement> = serde_json::from_slice(&data)?;
        let mut inner = self.inner.write().unwrap();
        for ent in entitlements {
            if !inner
                .iter()
                .any(|e| e.user_id == ent.user_id && e.sku_id == ent.sku_id)
            {
                inner.push(ent);
            }
        }
        Ok(())
    }
}

#[derive(Debug)]
pub enum StoreError {
    Io(std::io::Error),
    Json(serde_json::Error),
}

impl std::fmt::Display for StoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StoreError::Io(e) => write!(f, "io error: {e}"),
            StoreError::Json(e) => write!(f, "json error: {e}"),
        }
    }
}

impl std::error::Error for StoreError {}

impl From<std::io::Error> for StoreError {
    fn from(err: std::io::Error) -> Self {
        StoreError::Io(err)
    }
}

impl From<serde_json::Error> for StoreError {
    fn from(err: serde_json::Error) -> Self {
        StoreError::Json(err)
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
    if let Ok(id) = UserId::parse_str(user) {
        store.grant(id, sku.to_string());
    }
}

pub async fn create_purchase(
    db: &DatabaseConnection,
    user_id: UserId,
    sku_id: &str,
) -> Result<i64, DbErr> {
    let sku = sku_id.to_string();
    db.transaction(move |txn| {
        let sku = sku.clone();
        Box::pin(async move {
            let purchase = db::purchases::ActiveModel {
                user_id: Set(user_id),
                sku_id: Set(sku),
                purchased_at: Set(Utc::now()),
                ..Default::default()
            };
            let res = db::purchases::Entity::insert(purchase).exec(txn).await?;
            Ok(res.last_insert_id)
        })
    })
    .await
    .map_err(|e| match e {
        TransactionError::Connection(err) | TransactionError::Transaction(err) => err,
    })
}

pub async fn grant_entitlement(
    db: &DatabaseConnection,
    user_id: UserId,
    sku_id: &str,
) -> Result<(), DbErr> {
    let sku = sku_id.to_string();
    db.transaction(move |txn| {
        let sku = sku.clone();
        Box::pin(async move {
            let ent = db::entitlements::ActiveModel {
                user_id: Set(user_id),
                sku_id: Set(sku),
                granted_at: Set(Utc::now()),
            };
            db::entitlements::Entity::insert(ent)
                .on_conflict(
                    OnConflict::columns([
                        db::entitlements::Column::UserId,
                        db::entitlements::Column::SkuId,
                    ])
                    .do_nothing()
                    .to_owned(),
                )
                .exec(txn)
                .await?;
            Ok(())
        })
    })
    .await
    .map_err(|e| match e {
        TransactionError::Connection(err) | TransactionError::Transaction(err) => err,
    })
}

pub async fn list_entitlements(
    db: &DatabaseConnection,
    user_id: &str,
) -> Result<Vec<String>, DbErr> {
    if let Ok(id) = UserId::parse_str(user_id) {
        let rows = db::entitlements::Entity::find()
            .filter(db::entitlements::Column::UserId.eq(id))
            .all(db)
            .await?;
        Ok(rows.into_iter().map(|e| e.sku_id).collect())
    } else {
        Ok(Vec::new())
    }
}

mod db {
    use super::*;

    pub mod purchases {
        use super::*;

        #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
        #[sea_orm(table_name = "purchases")]
        pub struct Model {
            #[sea_orm(primary_key)]
            pub id: i64,
            pub user_id: Uuid,
            pub sku_id: String,
            pub purchased_at: DateTimeUtc,
        }

        #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
        pub enum Relation {}

        impl ActiveModelBehavior for ActiveModel {}
    }

    pub mod entitlements {
        use super::*;

        #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
        #[sea_orm(table_name = "entitlements")]
        pub struct Model {
            #[sea_orm(primary_key, auto_increment = false)]
            pub user_id: Uuid,
            #[sea_orm(primary_key, auto_increment = false)]
            pub sku_id: String,
            pub granted_at: DateTimeUtc,
        }

        #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
        pub enum Relation {}

        impl ActiveModelBehavior for ActiveModel {}
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

    #[test]
    fn grant_ignores_duplicates() {
        let store = EntitlementStore::default();
        let user = UserId::new_v4();
        store.grant(user, "pro".to_string());
        store.grant(user, "pro".to_string());
        assert_eq!(store.list(&user.to_string()).len(), 1);
    }

    #[test]
    fn load_ignores_duplicates() {
        let store = EntitlementStore::default();
        let user = UserId::new_v4();
        let ent = Entitlement {
            user_id: user,
            sku_id: "pro".to_string(),
            granted_at: Utc::now(),
        };
        let data = vec![ent.clone(), ent];
        let path = std::env::temp_dir().join("entitlements_test.json");
        std::fs::write(&path, serde_json::to_string(&data).unwrap()).unwrap();
        store.load(&path).unwrap();
        assert_eq!(store.list(&user.to_string()).len(), 1);
        let _ = std::fs::remove_file(path);
    }
}
