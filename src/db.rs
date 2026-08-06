use std::env;
use sqlx::{Sqlite, SqlitePool, migrate::MigrateDatabase};


pub async fn initialize_database() -> SqlitePool {
    let db_url = env::var("DATABASE_URL").expect("DATABASE_URL not configured in .env file");
    if !Sqlite::database_exists(&db_url).await.unwrap_or(false) {
        println!("Creating database {}", &db_url);
        match Sqlite::create_database(&db_url).await {
            Ok(_) => println!("Create db success"),
            Err(error) => panic!("error: {}", error),
        }
    } else {
        println!("Database already exists");
    }

    let pool: SqlitePool = SqlitePool::connect(&db_url).await.unwrap();

    sqlx::migrate!("./migrations").run(&pool).await.expect("Migrations has failed");

    pool
    
}