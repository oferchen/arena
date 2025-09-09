use chrono::Utc;
use sea_orm::{
    ActiveValue::Set, DatabaseConnection, QueryFilter, TransactionError, TransactionTrait,
    entity::prelude::*, sea_query::OnConflict,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
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

    pub fn all(&self) -> &[Sku] {
        &self.skus
    }
}

#[derive(Serialize, Deserialize)]
pub struct EntitlementList {
    pub entitlements: Vec<String>,
}

pub fn initiate_purchase(_user: &str, sku: &str) -> String {
    format!("session_{sku}")
}

pub async fn create_purchase(
    db: &DatabaseConnection,
    user_id: UserId,
    sku_id: &str,
) -> Result<Uuid, DbErr> {
    let sku = sku_id.to_string();
    let id = Uuid::new_v4();
    db.transaction(move |txn| {
        let sku = sku.clone();
        let id = id;
        Box::pin(async move {
            let purchase = db::purchases::ActiveModel {
                id: Set(id),
                player_id: Set(user_id.to_string()),
                sku: Set(sku),
                created_at: Set(Utc::now()),
            };
            db::purchases::Entity::insert(purchase).exec(txn).await?;
            Ok(id)
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
                player_id: Set(user_id.to_string()),
                sku: Set(sku),
                granted_at: Set(Utc::now()),
            };
            db::entitlements::Entity::insert(ent)
                .on_conflict(
                    OnConflict::columns([
                        db::entitlements::Column::PlayerId,
                        db::entitlements::Column::Sku,
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
    let rows = db::entitlements::Entity::find()
        .filter(db::entitlements::Column::PlayerId.eq(user_id))
        .all(db)
        .await?;
    Ok(rows.into_iter().map(|e| e.sku).collect())
}

mod db {
    use super::*;

    pub mod purchases {
        use super::*;

        #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
        #[sea_orm(table_name = "purchases")]
        pub struct Model {
            #[sea_orm(primary_key, auto_increment = false)]
            pub id: Uuid,
            pub player_id: String,
            pub sku: String,
            pub created_at: DateTimeUtc,
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
            pub player_id: String,
            #[sea_orm(primary_key, auto_increment = false)]
            pub sku: String,
            pub granted_at: DateTimeUtc,
        }

        #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
        pub enum Relation {}

        impl ActiveModelBehavior for ActiveModel {}
    }
}
