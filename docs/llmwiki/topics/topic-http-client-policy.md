---
title: HTTP Client Policy
status: verified
updated: 2026-06-10
---

# HTTP Client Policy

## Rule: Single HTTP Client — may_http Only

**Sesame-IDAM services are may coroutines. They share a single runtime.** Using `reqwest` (built on `tokio`) or any other async runtime's HTTP client would require a separate runtime threadpool, breaking the may model and wasting resources.

Every outbound HTTP call — including cross-service authz calls, JWKS fetching, and Redis HTTP APIs — must use the `may_http` client.

## What's Banned

| Client | Reason |
|--------|--------|
| `reqwest` | Built on `tokio`. Requires separate runtime. |
| `hyper` (direct) | Lower-level; `may_http` is the correct abstraction. |
| `surf`, `ureq`, `isahc` | Unrelated runtimes, no may integration. |
| Any `tokio::spawn` for background tasks | Background tasks must use `may::task::spawn` or the may runtime. |

## What's Allowed

- `may_http::client::Client` — all outbound HTTP
- `may::task::spawn` — background/coroutine tasks
- Redis via `may_redis` (separate skill)

## Code Anchors

- `may_http` → `git = "https://github.com/rust-may/may_http.git"` (not crates.io)
- `may` → `git = "https://github.com/microscaler/may.git"` (fork)
- `jwks_cache/cache.rs` — **migrated** to `may_http::client::HttpClient` (2026-07-06). NOTE: `may_http` pins `http = "0.2"`, so `Uri` values passed to it must come from the workspace `http_legacy` alias, not `http` 1.x.
- `token_versioning/subscriber.rs` — **migrated** (uses `arc_swap` + may; no `tokio::spawn` remains)
- `token_versioning/version_store.rs` — **migrated** (blocking redis client, no `tokio::spawn`)
- `fallback_cache/` — `reqwest` removed; `tokio` only in tests
