use std::{collections::HashSet, ops::Deref, sync::Arc};

use axum::extract::FromRef;
use axum_extra::extract::cookie::Key;
use chrono::NaiveDateTime;
use serde::Serialize;
use sqlx::{SqlitePool, prelude::FromRow};



#[derive(Clone)]
pub struct AppState(pub Arc<AppStateInner>);

pub struct AppStateInner {
    pub db: SqlitePool,
    pub sqids: sqids::Sqids,
    pub domain_blocklist: HashSet<String>,
    pub slug_blocklist: HashSet<String>,
    pub cookie_key: Key,
    pub admin_password: String
}

impl FromRef<AppState> for Key {
    fn from_ref(state: &AppState) -> Self {
        state.0.cookie_key.clone()
    }
}

impl Deref for AppState {
    type Target = AppStateInner;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug, FromRow, Serialize)]
pub struct ClickRecord{
    pub slug: String,
    pub referrer: String,
    pub user_agent: String,
    pub ip: String,
    pub created_at: Option<NaiveDateTime>
}