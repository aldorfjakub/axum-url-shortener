use std::{collections::HashSet, ops::Deref, sync::Arc};

use axum::extract::FromRef;
use axum_extra::extract::cookie::Key;
use chrono::NaiveDateTime;
use sqlx::{SqlitePool, prelude::FromRow};



#[derive(Clone)]
pub struct AppState(pub Arc<AppStateInner>);

pub struct AppStateInner {
    pub db: SqlitePool,
    pub sqids: sqids::Sqids,
    pub blocklist: HashSet<String>,
    pub cookie_key: Key,
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

#[derive(Debug, FromRow)]
pub struct LinkRecord{
    pub slug: String,
    pub original_url: String,
    pub password_hash: Option<String>,
    pub created_at: NaiveDateTime
}