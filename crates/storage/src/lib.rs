use anyhow::Result;
use sea_orm::{ConnectOptions, Database, DatabaseConnection};

/// Connect to the database and return a SeaORM [`DatabaseConnection`].
pub async fn connect(db_url: &str, max_connections: u32) -> Result<DatabaseConnection> {
    let mut opts = ConnectOptions::new(db_url.to_owned());
    opts.max_connections(max_connections);
    let db = Database::connect(opts).await?;
    Ok(db)
}
