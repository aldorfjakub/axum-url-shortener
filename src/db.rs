use std::{env, str::FromStr, time::Duration};
use sqlx::{Sqlite, SqlitePool, migrate::MigrateDatabase, sqlite::{SqliteConnectOptions, SqliteJournalMode}};


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
    let opts = SqliteConnectOptions::from_str(&db_url).unwrap()
    .journal_mode(SqliteJournalMode::Wal)
    .busy_timeout(Duration::from_secs(5))
    .foreign_keys(true);

    let pool: SqlitePool = SqlitePool::connect_with(opts).await.unwrap();

    sqlx::migrate!("./migrations").run(&pool).await.expect("Migrations has failed");

    pool
    
}