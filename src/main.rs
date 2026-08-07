use std::{collections::HashSet, env, net::SocketAddr, sync::Arc};

use axum_extra::extract::cookie::Key;
use dotenvy::dotenv;
use sqids::Sqids;

use crate::models::{AppState, AppStateInner};

use base64::prelude::*;

mod db;
mod models;
mod routes;

#[tokio::main]
async fn main() {
    dotenv().ok();
    let db = db::initialize_database().await;
    let sqids = Sqids::builder()
        .min_length(4)
        .alphabet(
            "pgQHqibXSKe4N06t7sPD3IuZc8C9EmylzjGMaVJFBWOR21wnoUYf5LxArhvkTd"
                .chars()
                .collect(),
        )
        .build()
        .expect("Failed to create Sqids instance");

    let domain_blocklist_path = std::env::var("DOMAIN_BLOCKLIST");
    if domain_blocklist_path.is_err() {
        println!(
            "DOMAIN_BLOCKLIST is not configured, all target domains will be allowed. To configure a blocklist, set the DOMAIN_BLOCKLIST environment variable to the path of a text file containing blocked domains."
        );
    }
    let domain_blocklist: HashSet<String> = match domain_blocklist_path {
        Ok(path) => std::fs::read_to_string(path)
            .unwrap()
            .lines()
            .map(|s| s.to_string())
            .collect(),
        Err(_) => HashSet::new(),
    };

    let slug_blocklist_path = std::env::var("SLUG_BLOCKLIST");
    if slug_blocklist_path.is_err() {
        println!(
            "SLUG_BLOCKLIST is not configured, all slugs will be allowed. To configure a blocklist, set the SLUG_BLOCKLIST environment variable to the path of a text file containing blocked slugs."
        );
    }

    let slug_blocklist: HashSet<String> = match slug_blocklist_path {
        Ok(path) => std::fs::read_to_string(path)
            .unwrap()
            .lines()
            .map(|s| s.to_string())
            .collect(),
        Err(_) => HashSet::new(),
    };

    let cookie_secret = env::var("COOKIE_SECRET")
        .expect("COOKIE_SECRET must be configured");

    let key_bytes = BASE64_STANDARD.decode(&cookie_secret).expect("COOKIE_SECRET must be a valid base64 string");

    let cookie_key = Key::try_from(key_bytes.as_slice()).expect("COOKIE_SECRET decoded bytes must be at least 64 bytes long");

    let admin_password= env::var("ADMIN_PASSWORD").expect("ADMIN_PASSWORD needs to be set in order to view link stats");
    let state = AppState(Arc::new(AppStateInner {
        db,
        sqids,
        domain_blocklist,
        slug_blocklist,
        cookie_key,
        admin_password
    }));

    let app = routes::app_routes(state);

    let ip_addr = std::env::var("BIND_ADDRESS").unwrap_or_else(|_| "127.0.0.1:3000".into());
    let listener = tokio::net::TcpListener::bind(&ip_addr).await.unwrap();
    println!("The app is served at {}", &ip_addr);
    let _ = axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await
    .unwrap();

    
}

async fn shutdown_signal() {
    // Handle Ctrl+C
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    // Handle SIGTERM
    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    // Non-Unix fallback
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {}
        _ = terminate => {}
    }

    println!("Shutdown signal received, finishing requests...");
}