use std::{env, net::SocketAddr, string};

use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng},
};
use axum::{
    Json, Router,
    extract::{ConnectInfo, Path, State},
    http::{HeaderMap, StatusCode},
    response::{Html, IntoResponse, Redirect},
    routing::{get, post},
};
use axum_extra::extract::{PrivateCookieJar, cookie::Cookie};
use constant_time_eq::constant_time_eq;
use serde::Deserialize;
use serde_json::json;

use crate::models::{AppState, ClickRecord};

async fn index() -> impl IntoResponse {
    Html(include_str!("../html/index.html"))
}

#[derive(Deserialize)]
pub struct CreateRequest {
    pub long_url: String,
    pub slug: Option<String>,
    pub password: Option<String>,
}
#[derive(Deserialize)]
pub struct PasswordVerifyRequest {
    pub password: String,
}

async fn link_creation_api(
    State(state): State<AppState>,
    Json(payload): Json<CreateRequest>,
) -> impl IntoResponse {
    let host = match url::Url::parse(&payload.long_url) {
        Ok(url) => url.host_str().map(|s| s.to_string()),
        Err(_) => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": "Invalid URL format." })),
            ));
        }
    };
    if host.is_none() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "Invalid URL format." })),
        ));
    }
    if state.domain_blocklist.contains(&host.unwrap()) {
        return Err((
            StatusCode::FORBIDDEN,
            Json(json!({ "error": "This URL is blocked." })),
        ));
    }

    let pass_hash: Option<String> = match payload.password {
        Some(pass) => {
            let salt = SaltString::generate(&mut OsRng);
            let argon2 = Argon2::default();
            let password_hash = argon2
                .hash_password(pass.as_bytes(), &salt)
                .map_err(|e| -> (StatusCode, Json<serde_json::Value>) {
                    println!("Error hashing password: {}", e);
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(json!({ "error": "Failed to hash password." })),
                    )
                })?
                .to_string();
            Some(password_hash)
        }
        None => None,
    };

    if payload.slug.is_some() {
        let custom_slug = payload.slug.as_ref().unwrap();

        // This leaves around 2 billion possible combinations, which should be enough for a small project like this.
        if custom_slug.len() < 7 {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": "Custom slug must be at least 7 characters long." })),
            ));
        } else if custom_slug.len() > 20 {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": "Custom slug must be at most 20 characters long." })),
            ));
        } else if !custom_slug.chars().all(|c| c.is_alphanumeric() || c == '-') {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(
                    json!({ "error": "Custom slug can only contain alphanumeric characters and dashes." }),
                ),
            ));
        } else if state.slug_blocklist.contains(custom_slug) {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": "This custom slug is banned from using it." })),
            ));
        }

        let _ = sqlx::query!(
            "INSERT INTO links (slug, original_url, password_hash) VALUES (?, ?, ?)",
            custom_slug,
            payload.long_url,
            pass_hash
        )
        .execute(&state.db)
        .await
        .map_err(|_| {
            (
                StatusCode::FORBIDDEN,
                Json(json!({ "error": "Custom slug is already taken." })),
            )
        })?;

        return Ok(Json(json!({ "slug": custom_slug })));
    }

    let mut tx = state.db.begin().await.map_err(|err| {
        println!("Error starting transaction: {}", err);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": "Failed to create short link." })),
        )
    })?;
    let mut slug = String::new();
    let mut slug_gen_attempts = 0;

    // Give multiple attempts to generate correct slug
    while slug.is_empty() {
        let result = match sqlx::query!("INSERT INTO l_counter DEFAULT VALUES")
            .execute(&mut *tx)
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
        slug = match state.sqids.encode(&[inserted_id as u64]) {
            Ok(s) => {
                slug_gen_attempts += 1;

                if state.slug_blocklist.contains(&s) {
                    String::new()
                } else {
                    s
                }
            }
            Err(err) => {
                println!("Error encoding slug: {}", err);
                slug_gen_attempts += 1;
                // Limit the attempts to prevent deadlock
                if slug_gen_attempts > 5 {
                    return Err((
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(json!({ "error": "Failed to create short link." })),
                    ));
                } else {
                    String::new()
                }
            }
        };
    }
    let _ = sqlx::query!(
        "INSERT INTO links (slug, original_url, password_hash) VALUES (?, ?, ?)",
        slug,
        payload.long_url,
        pass_hash
    )
    .execute(&mut *tx)
    .await
    .map_err(|err| {
        println!("Error inserting into links: {}", err);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": "Failed to create short link." })),
        )
    })?;

    tx.commit().await.map_err(|err| {
        println!("Error committing transaction: {}", err);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": "Failed to create short link." })),
        )
    })?;

    Ok(Json(json!({ "slug": slug })))
}

async fn link_redirect(
    State(state): State<AppState>,
    Path(short): Path<String>,
    jar: PrivateCookieJar,
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> impl IntoResponse {
    let result = sqlx::query!(
        "SELECT original_url, password_hash FROM links WHERE slug = ?",
        short
    )
    .fetch_one(&state.db)
    .await;
    let record = match result {
        Ok(record) => record,
        Err(err) => {
            println!("Error fetching original_url: {}", err);
            return Err((
                StatusCode::NOT_FOUND,
                Json(json!({ "error": "Short link not found." })),
            ));
        }
    };

    let authorized = match jar.get(&short) {
        Some(cookie) => cookie
            .value()
            .parse::<i64>()
            .map(|expiry| chrono::Utc::now().timestamp() < expiry)
            .unwrap_or(false),
        None => false,
    };

    if record.password_hash.is_some() && !authorized {
        return Ok((
            StatusCode::OK,
            Html(include_str!("../html/password-input.html")),
        )
            .into_response());
    }

    // Log the click, refferer, user agent, ip address
    let user_agent = headers
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("Unknown");
    let referrer = headers
        .get("referer")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("Direct");
    let ip_addr = addr.ip().to_string();

    println!(
        "Registered click for slug: {}, referrer: {}, user_agent: {}, ip: {}",
        short, referrer, user_agent, ip_addr
    );
    let _ = sqlx::query!(
        "INSERT INTO clicks (slug, referrer, user_agent, ip) VALUES (?, ?, ?, ?)",
        short,
        referrer,
        user_agent,
        ip_addr
    )
    .execute(&state.db)
    .await;

    Ok(Redirect::to(&record.original_url).into_response())
}

async fn link_stats_page(jar: PrivateCookieJar) -> impl IntoResponse {
    let authorized = match jar.get("admin") {
        Some(cookie) => cookie
            .value()
            .parse::<i64>()
            .map(|expiry| chrono::Utc::now().timestamp() < expiry)
            .unwrap_or(false),
        None => false,
    };

    if !authorized {
        return Html(include_str!("../html/password-input-stats.html"));
    }

    Html(include_str!("../html/stats.html"))
}

async fn password_verify_api(
    State(state): State<AppState>,
    Path(short): Path<String>,
    jar: PrivateCookieJar,
    Json(payload): Json<PasswordVerifyRequest>,
) -> impl IntoResponse {
    let record = match sqlx::query!(
        "SELECT original_url, password_hash FROM links WHERE slug = ?",
        short
    )
    .fetch_one(&state.db)
    .await
    {
        Ok(record) => record,
        Err(_) => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(json!({ "error": "Short link not found." })),
            ));
        }
    };

    let Some(hash) = record.password_hash else {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "This link is not password protected." })),
        ));
    };

    let argon2 = Argon2::default();
    let valid = argon2
        .verify_password(
            payload.password.as_bytes(),
            &PasswordHash::new(&hash).unwrap(),
        )
        .is_ok();

    if !valid {
        return Err((
            StatusCode::FORBIDDEN,
            Json(json!({ "error": "Invalid password." })),
        ));
    }

    // Issue cookie with a 5-minute expiry
    let expiry = chrono::Utc::now().timestamp() + 300;
    let cookie = Cookie::build((short.clone(), expiry.to_string()))
        .path("/")
        .http_only(true)
        .secure(false)
        .same_site(axum_extra::extract::cookie::SameSite::Strict)
        .build();
    let jar = jar.add(cookie);

    Ok((jar, Json(json!({ "success": true, "slug": short }))).into_response())
}

async fn admin_acces_verify_api(
    State(state): State<AppState>,
    jar: PrivateCookieJar,
    Json(payload): Json<PasswordVerifyRequest>,
) -> impl IntoResponse {
    if constant_time_eq(&payload.password.into_bytes() , &state.admin_password.clone().into_bytes())
    {
        return Err((
            StatusCode::FORBIDDEN,
            Json(json!({"error": "Invalid password"})),
        ));
    }
    let expiry = chrono::Utc::now().timestamp() + 1800;
    let cookie = Cookie::build(("admin", expiry.to_string()))
        .path("/")
        .http_only(true)
        .secure(false)
        .same_site(axum_extra::extract::cookie::SameSite::Strict)
        .build();

    let jar = jar.add(cookie);

    Ok((jar, Json(json!({"success": true}))).into_response())
}

async fn link_stats_api(
    State(state): State<AppState>,
    Path(short): Path<String>,
    jar: PrivateCookieJar,
) -> impl IntoResponse {
    let authorized = match jar.get("admin") {
        Some(cookie) => cookie
            .value()
            .parse::<i64>()
            .map(|expiry| chrono::Utc::now().timestamp() < expiry)
            .unwrap_or(false),
        None => false,
    };

    if !authorized {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({"error": "Not authorized to view stats."})),
        );
    }
    match sqlx::query!("SELECT * FROM links WHERE slug = ?", short)
        .fetch_one(&state.db)
        .await
    {
        Ok(_) => (),
        Err(_) => {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({"error": "Short link not found."})),
            );
        }
    }
    let rows = match sqlx::query_as!(
        ClickRecord,
        "SELECT slug, referrer, user_agent, ip, created_at  FROM clicks WHERE slug = ? ORDER BY clicks.id DESC",
        short
    )
    .fetch_all(&state.db)
    .await
    {
        Ok(v) => v,
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "Failed to fetch click stats." })),
            );
        }
    };

    (StatusCode::OK, Json(json!(rows)))
}
pub fn app_routes(state: AppState) -> Router<()> {
    let api_routes = Router::new()
        .route("/", get(index))
        .route("/{short}", get(link_redirect))
        .route("/{short}/verify", post(password_verify_api))
        .route("/{short}/stats", get(link_stats_page))
        .route("/api/shorten", post(link_creation_api))
        .route("/api/admin/verify", post(admin_acces_verify_api))
        .route("/api/stats/{short}", get(link_stats_api))
        .with_state(state);

    api_routes
}
