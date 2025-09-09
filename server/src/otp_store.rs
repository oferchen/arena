use chrono::{DateTime, Utc};
use sea_orm::{ActiveValue::Set, DbErr, QueryFilter, entity::prelude::*};

pub async fn insert_otp(
    db: &DatabaseConnection,
    email_hash: &str,
    code: &str,
    expires_at: DateTime<Utc>,
) -> Result<(), DbErr> {
    let active = email_otps::ActiveModel {
        email_hash: Set(email_hash.to_owned()),
        code: Set(code.to_owned()),
        expires_at: Set(expires_at.into()),
    };
    email_otps::Entity::insert(active).exec(db).await?;
    Ok(())
}

pub async fn fetch_otp(
    db: &DatabaseConnection,
    email_hash: &str,
) -> Result<Option<(String, DateTime<Utc>)>, DbErr> {
    if let Some(row) = email_otps::Entity::find()
        .filter(email_otps::Column::EmailHash.eq(email_hash))
        .one(db)
        .await?
    {
        return Ok(Some((row.code, row.expires_at.into())));
    }
    Ok(None)
}

pub async fn delete_otp(db: &DatabaseConnection, email_hash: &str) -> Result<(), DbErr> {
    email_otps::Entity::delete_many()
        .filter(email_otps::Column::EmailHash.eq(email_hash))
        .exec(db)
        .await?;
    Ok(())
}

mod email_otps {
    use sea_orm::entity::prelude::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "email_otps")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub email_hash: String,
        pub code: String,
        pub expires_at: DateTimeWithTimeZone,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}
