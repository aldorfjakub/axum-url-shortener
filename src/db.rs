use std::sync::Arc;

use sqlx::{Pool, Sqlite, SqlitePool, migrate::MigrateDatabase};

const DB_URL: &str = "sqlite://sqlite.db";


pub async fn initialize_database() -> SqlitePool {
    if !Sqlite::database_exists(DB_URL).await.unwrap_or(false) {
        println!("Creating database {}", DB_URL);
        match Sqlite::create_database(DB_URL).await {
            Ok(_) => println!("Create db success"),
            Err(error) => panic!("error: {}", error),
        }
    } else {
        println!("Database already exists");
    }

    let pool: SqlitePool = SqlitePool::connect(DB_URL).await.unwrap();

    sqlx::migrate!("./migrations").run(&pool).await.expect("Migrations has failed");

    pool
    
}