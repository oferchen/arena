use sea_orm::entity::prelude::*;
use serde_json::Value as JsonValue;
use uuid::Uuid;
use chrono::{DateTime, Utc};

type DateTimeUtc = DateTime<Utc>;

pub mod login_tokens {
    use super::*;
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "login_tokens")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub token: String,
        pub player_id: String,
        pub created_at: DateTimeUtc,
        pub expires_at: DateTimeUtc,
    }
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
    impl ActiveModelBehavior for ActiveModel {}
}

pub mod leaderboards {
    use super::*;
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "leaderboards")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
    }
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
    impl ActiveModelBehavior for ActiveModel {}
}

pub mod runs {
    use super::*;
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "runs")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        pub leaderboard: Uuid,
        pub player_id: String,
        pub replay_path: String,
        pub created_at: DateTimeUtc,
        pub flagged: bool,
        pub replay_index: i64,
    }
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        #[sea_orm(has_many = "super::scores::Entity")]
        Scores,
    }
    impl Related<super::scores::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::Scores.def()
        }
    }
    impl ActiveModelBehavior for ActiveModel {}
}

pub mod scores {
    use super::*;
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "scores")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        pub run: Uuid,
        pub leaderboard: Uuid,
        pub player_id: String,
        pub points: i32,
        pub created_at: DateTimeUtc,
        pub verified: bool,
    }
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        #[sea_orm(
            belongs_to = "super::runs::Entity",
            from = "Column::Run",
            to = "runs::Column::Id"
        )]
        Runs,
    }
    impl Related<super::runs::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::Runs.def()
        }
    }
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

pub mod levels {
    use super::*;
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "levels")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        pub name: String,
        pub data: String,
    }
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
    impl ActiveModelBehavior for ActiveModel {}
}

pub mod analytics_events {
    use super::*;
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "analytics_events")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub ts: DateTimeUtc,
        pub player_id: Option<String>,
        pub session_id: Option<Uuid>,
        pub kind: String,
        pub payload_json: Option<JsonValue>,
    }
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
    impl ActiveModelBehavior for ActiveModel {}
}

pub mod analytics_rollups {
    use super::*;
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "analytics_rollups")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub bucket_start: DateTimeUtc,
        #[sea_orm(primary_key, auto_increment = false)]
        pub kind: String,
        pub value: f64,
    }
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
    impl ActiveModelBehavior for ActiveModel {}
}

pub mod mail_outbox {
    use super::*;
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "mail_outbox")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i64,
        pub recipient: String,
        pub subject: String,
        pub body: String,
        pub created_at: DateTimeUtc,
        pub sent_at: Option<DateTimeUtc>,
        pub error: Option<String>,
    }
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
    impl ActiveModelBehavior for ActiveModel {}
}

pub mod jobs {
    use super::*;
    #[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum)]
    #[sea_orm(rs_type = "String", db_type = "Text")]
    pub enum JobStatus {
        #[sea_orm(string_value = "pending")]
        Pending,
        #[sea_orm(string_value = "running")]
        Running,
        #[sea_orm(string_value = "done")]
        Done,
        #[sea_orm(string_value = "failed")]
        Failed,
    }
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "jobs")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i64,
        pub kind: String,
        pub payload: String,
        pub status: JobStatus,
        pub attempts: i32,
        pub run_at: DateTimeUtc,
        pub created_at: DateTimeUtc,
        pub updated_at: DateTimeUtc,
    }
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
    impl ActiveModelBehavior for ActiveModel {}
}

pub mod nodes {
    use super::*;
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "nodes")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        pub region: String,
        pub last_seen: DateTimeUtc,
        pub info: JsonValue,
    }
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
    impl ActiveModelBehavior for ActiveModel {}
}
