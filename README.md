# URL Shortener

A URL shortener built with Rust (Axum) and SQLite. Create short links, optionally
protect them behind a password, and view per-link click analytics.
Runs at `u.d15.club` (hosted on my personal server).

## Features

- **Short link generation** — ids produced by [sqids](https://github.com/sqids/sqids-rust) from a database counter; no collision-retry loops
- **Custom slugs** — 7–20 characters, restricted charset (`[A-Za-z0-9-]`), slug blocklist
- **Password-protected links** — argon2-hashed; access granted via a short-lived signed cookie
- **Click analytics** — records referrer, user agent, and client IP per click, newest first
- **Admin-gated stats** — `/{slug}/stats` requires a password (constant-time comparison, 30-minute session)
- **Domain + slug blocklists** — loaded from text files at startup
- **Server-rendered UI** — plain HTML/JS embedded via `include_str!`, no build step

## Architecture

The app is a single Axum process backed by one SQLite database (WAL mode,
`busy_timeout`, `foreign_keys` enforced). It is served behind a reverse proxy
that terminates TLS and rate-limits link creation.

```
Browser ──HTTPS──► nginx ──proxy──► Axum (127.0.0.1:3000) ──► SQLite (WAL)
```

- A static `/health` endpoint reports DB liveness for load balancers/orchestrators.
- Client IPs are read from `X-Forwarded-For` (with a socket-address fallback when no proxy is present).
- Cookies are signed (`COOKIE_SECRET`) and can be marked `Secure` via env.

## API

| Method | Path | Description |
|--------|------|-------------|
| `POST` | `/api/shorten` | Create a short link (`{ "long_url", "slug?", "password?" }`) |
| `GET` | `/{slug}` | Redirect to the original URL |
| `POST` | `/{slug}/verify` | Verify a link password, issue session cookie |
| `GET` | `/{slug}/stats` | Stats page (admin login required) |
| `GET` | `/api/stats/{slug}` | Click records as JSON (admin cookie required) |
| `POST` | `/api/admin/verify` | Admin login for stats |
| `GET` | `/health` | Health check (DB liveness) |

## Tech

Rust (edition 2024) · [Axum 0.8](https://github.com/tokio-rs/axum) · SQLx 0.9
(SQLite) · Tokio · Argon2 · Sqids · axum-extra (signed cookies)

## Running locally

```bash
cp .env.example .env    # fill in COOKIE_SECRET, ADMIN_PASSWORD, blocklist paths
cargo run               # serves on 127.0.0.1:3000
```

Migrations run automatically on startup. The DB path, blocklist paths, and
secrets are all configurable via env — nothing is hardcoded to a working
directory.

## Deployment

Deployed on a Hetzner VPS as a hardened `systemd` service (dedicated user,
`ProtectSystem`, `NoNewPrivileges`). nginx terminates TLS (Let's Encrypt) and
rate-limits `/api/shorten`. The database lives in `/var/lib/url-shortener`,
separate from the code so updates don't touch data. Builds use
`SQLX_OFFLINE=true` with the committed `.sqlx` query cache.
