use chrono::{DateTime, Utc};
use scylla::{IntoTypedRows, Session};

pub async fn insert_otp(
    db: &Session,
    email_hash: &str,
    code: &str,
    expires_at: DateTime<Utc>,
) {
    let query =
        "INSERT INTO email_otps (email_hash, code, expires_at) VALUES (?, ?, ?)";
    let _ = db
        .query(query, (email_hash.to_string(), code.to_string(), expires_at))
        .await;
}

pub async fn fetch_otp(
    db: &Session,
    email_hash: &str,
) -> Option<(String, DateTime<Utc>)> {
    let query = "SELECT code, expires_at FROM email_otps WHERE email_hash = ?";
    if let Ok(res) = db.query(query, (email_hash.to_string(),)).await {
        if let Some(rows) = res.rows {
            let mut rows = rows.into_typed::<(String, DateTime<Utc>)>();
            if let Some(row) = rows.next() {
                if let Ok(data) = row {
                    return Some(data);
                }
            }
        }
    }
    None
}

pub async fn delete_otp(db: &Session, email_hash: &str) {
    let query = "DELETE FROM email_otps WHERE email_hash = ?";
    let _ = db.query(query, (email_hash.to_string(),)).await;
}

