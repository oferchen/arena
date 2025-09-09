use chrono::Utc;
use serde::{Deserialize, Serialize};
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
