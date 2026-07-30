use std::collections::HashSet;

use sqids::Sqids;

use crate::models::AppState;


mod db;
mod models;
mod routes;

#[tokio::main]
async fn main() {
    let db =  db::initialize_database().await;
    let sqids = Sqids::builder().min_length(4).alphabet("pgQHqibXSKe4N06t7sPD3IuZc8C9EmylzjGMaVJFBWOR21wnoUYf5LxArhvkTd".chars().collect()).build().expect("Failed to create Sqids instance");

    let blocklist: HashSet<String> = std::fs::read_to_string("data/blocklist.txt").unwrap().lines().map(|s| s.to_string()).collect();

    let state = AppState{db,sqids, blocklist};

    let app = routes::app_routes(state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();
    println!("The app is served at 127.0.0.1:3000");
    let _ = axum::serve(listener, app).await;
    
    println!("Hello, world!");
}
