use anyhow::Result;
use sqlx::{PgPool, postgres::PgPoolOptions};

pub async fn connect(db_url: &str) -> Result<PgPool> {
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(db_url)
        .await?;
    Ok(pool)
}
