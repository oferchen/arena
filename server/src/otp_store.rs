use chrono::{DateTime, Utc};
use sea_orm::{DatabaseConnection, DbBackend, Statement, TryGetable};

pub async fn insert_otp(
    db: &DatabaseConnection,
    email_hash: &str,
    code: &str,
    expires_at: DateTime<Utc>,
) {
    let stmt = Statement::from_sql_and_values(
        DbBackend::Postgres,
        "INSERT INTO email_otps (email_hash, code, expires_at) VALUES ($1, $2, $3)",
        vec![email_hash.into(), code.into(), expires_at.into()],
    );
    let _ = db.execute(stmt).await;
}

pub async fn fetch_otp(
    db: &DatabaseConnection,
    email_hash: &str,
) -> Option<(String, DateTime<Utc>)> {
    let stmt = Statement::from_sql_and_values(
        DbBackend::Postgres,
        "SELECT code, expires_at FROM email_otps WHERE email_hash = $1",
        vec![email_hash.into()],
    );
    if let Ok(Some(row)) = db.query_one(stmt).await {
        if let (Ok(code), Ok(expires_at)) = (
            row.try_get::<String>("code"),
            row.try_get::<DateTime<Utc>>("expires_at"),
        ) {
            return Some((code, expires_at));
        }
    }
    None
}

pub async fn delete_otp(db: &DatabaseConnection, email_hash: &str) {
    let stmt = Statement::from_sql_and_values(
        DbBackend::Postgres,
        "DELETE FROM email_otps WHERE email_hash = $1",
        vec![email_hash.into()],
    );
    let _ = db.execute(stmt).await;
}

