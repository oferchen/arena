use ::payments::UserId;
use chrono::Utc;
use sea_orm::{entity::prelude::*, DatabaseConnection, OnConflict, QueryFilter};

#[derive(Clone, Default)]
pub struct EntitlementStore {
    db: Option<DatabaseConnection>,
}

impl EntitlementStore {
    pub fn new(db: Option<DatabaseConnection>) -> Self {
        Self { db }
    }

    pub async fn grant(&self, user_id: UserId, sku_id: String) {
        if let Some(db) = &self.db {
            let active = entitlements::ActiveModel {
                user_id: Set(user_id),
                sku_id: Set(sku_id.clone()),
                granted_at: Set(Utc::now()),
            };
            let _ = entitlements::Entity::insert(active)
                .on_conflict(
                    OnConflict::columns([
                        entitlements::Column::UserId,
                        entitlements::Column::SkuId,
                    ])
                    .do_nothing()
                    .to_owned(),
                )
                .exec(db)
                .await;
        }
    }

    pub async fn list(&self, user_id: &str) -> Vec<String> {
        if let Some(db) = &self.db {
            if let Ok(id) = UserId::parse_str(user_id) {
                if let Ok(rows) = entitlements::Entity::find()
                    .filter(entitlements::Column::UserId.eq(id))
                    .all(db)
                    .await
                {
                    return rows.into_iter().map(|e| e.sku_id).collect();
                }
            }
        }
        Vec::new()
    }
}

mod entitlements {
    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(
        table_name = "entitlements_by_user",
        primary_key = (user_id, sku_id)
    )]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub user_id: Uuid,
        #[sea_orm(primary_key, auto_increment = false)]
        pub sku_id: String,
        pub granted_at: DateTimeWithTimeZone,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

mod purchases {
    use sea_orm::entity::prelude::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "purchases_by_user")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i32,
        pub user_id: Uuid,
        pub sku_id: String,
        pub purchased_at: DateTimeWithTimeZone,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

