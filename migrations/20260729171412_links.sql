-- Add migration script here
CREATE TABLE IF NOT EXISTS links (
    slug TEXT PRIMARY KEY,
    original_url TEXT NOT NULL,
    password_hash TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);