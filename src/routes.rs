use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    response::{Html, IntoResponse},
    routing::{get, post},
};
use serde::Deserialize;
use serde_json::json;
use sqids::Sqids;

use crate::models::AppState;

async fn index() -> impl IntoResponse {
    Html(include_str!("../html/index.html"))
}

#[derive(Deserialize)]
pub struct CreateRequest {
    pub long_url: String,
}

async fn link_creation_api(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<CreateRequest>,
) -> impl IntoResponse {
    let result = match sqlx::query!("INSERT INTO l_counter DEFAULT VALUES")
        .execute(&state.db)
        .await
    {
        Ok(res) => res,
        Err(err) => {
            println!("Error inserting into l_counter: {}", err);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "Failed to create short link." })),
            ));
        }
    };
    let inserted_id = result.last_insert_rowid();
    let slug = match state.sqids.encode(&[inserted_id as u64]) {
        Ok(s) => s,
        Err(err) => {
            println!("Error encoding slug: {}", err);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "Failed to create short link." })),
            ));
        }
    };
    let result = sqlx::query!(
        "INSERT INTO links (slug, original_url) VALUES (?, ?)",
        slug,
        payload.long_url
    )
    .execute(&state.db)
    .await;

    match result {
        Ok(_) => {
            return Ok((
                StatusCode::OK,
                Json(json!({
                    "slug" : slug
                })),
            ));
        }
        Err(err) => {
            println!("Error inserting into links: {}", err);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "Failed to create short link." })),
            ));
        }
    }
}

async fn link_redirect(State(state): State<Arc<AppState>>, Path(short): Path<String>) -> impl IntoResponse {
    let result = sqlx::query!("SELECT original_url FROM links WHERE slug = ?", short).fetch_one(&state.db).await;
    match result {
        Ok (record) => return Ok((StatusCode::FOUND, axum::response::Redirect::to(&record.original_url))),
        Err (err) => {
            println!("Error fetching original_url: {}", err);
            return Err((
                StatusCode::NOT_FOUND,
                Json(json!({ "error": "Short link not found." })),
            ));
        }
    }
}

pub fn app_routes(state: AppState) -> Router<()> {
    let state = Arc::new(state);
    let api_routes = Router::new()
        .route("/", get(index))
        .route("/{short}", get(link_redirect))
        .route("/api/shorten", post(link_creation_api))
        .with_state(state);

    api_routes
}
