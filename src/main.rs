use std::sync::Arc;

use crate::models::AppState;


mod db;
mod models;
mod routes;

#[tokio::main]
async fn main() {
    let db =  db::initialize_database().await;
    let state = AppState{db};

    let app = routes::app_routes(state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();
    println!("The app is served at 127.0.0.1:3000");
    let _ = axum::serve(listener, app).await;
    
    println!("Hello, world!");
}
