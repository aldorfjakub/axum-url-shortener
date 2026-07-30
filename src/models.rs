use std::collections::HashSet;

use chrono::NaiveDateTime;
use sqlx::{SqlitePool, prelude::FromRow};

pub struct AppState{
    pub db: SqlitePool,
    pub sqids: sqids::Sqids,
    pub blocklist: HashSet<String>
}

#[derive(Debug, FromRow)]
pub struct LinkRecord{
    pub slug: String,
    pub original_url: String,
    pub password_hash: Option<String>,
    pub created_at: NaiveDateTime
}