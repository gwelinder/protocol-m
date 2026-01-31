//! Database connection and pool management.

use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

/// Creates a database connection pool.
///
/// # Arguments
/// * `database_url` - PostgreSQL connection string
///
/// # Returns
/// A configured PgPool ready for use.
pub async fn create_pool(database_url: &str) -> Result<PgPool, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(10)
        .connect(database_url)
        .await
}

/// Runs all pending migrations.
pub async fn run_migrations(pool: &PgPool) -> Result<(), sqlx::migrate::MigrateError> {
    sqlx::migrate!("./migrations").run(pool).await
}
