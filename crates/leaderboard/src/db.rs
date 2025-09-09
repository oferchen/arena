use sea_orm::entity::prelude::*;

pub mod runs {
    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "runs")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: Uuid,
        pub leaderboard_id: Uuid,
        pub player_id: Uuid,
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
        #[sea_orm(primary_key)]
        pub id: Uuid,
        pub run_id: Uuid,
        pub leaderboard_id: Uuid,
        pub player_id: Uuid,
        pub points: i32,
        pub created_at: DateTimeUtc,
        pub verified: bool,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        #[sea_orm(belongs_to = "super::runs::Entity", from = "Column::RunId", to = "runs::Column::Id")]
        Runs,
    }

    impl Related<super::runs::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::Runs.def()
        }
    }

    impl ActiveModelBehavior for ActiveModel {}
}

pub mod purchases {
    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "purchases")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: Uuid,
        pub user_id: Uuid,
        pub sku: String,
        pub created_at: DateTimeUtc,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}
