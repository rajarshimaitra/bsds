pub mod models;

use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};
use std::str::FromStr;

pub async fn connect(database_url: &str) -> SqlitePool {
    let options = SqliteConnectOptions::from_str(database_url)
        .expect("invalid database URL")
        .create_if_missing(true);

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await
        .expect("failed to connect to SQLite database");

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("database migration failed");

    sqlx::query("PRAGMA journal_mode=WAL;")
        .execute(&pool)
        .await
        .expect("failed to set WAL mode");

    sqlx::query("PRAGMA foreign_keys=ON;")
        .execute(&pool)
        .await
        .expect("failed to enable foreign keys");

    pool
}
