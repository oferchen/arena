use anyhow::Result;
use sea_orm::{Database, DatabaseConnection};

/// Connect to the database and return a SeaORM [`DatabaseConnection`].
pub async fn connect(db_url: &str) -> Result<DatabaseConnection> {
    let db = Database::connect(db_url).await?;
    Ok(db)
}
