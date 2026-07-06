---
title: HTTP Client Policy
status: verified
updated: 2026-07-06
---

# HTTP Client Policy

## Rule: Single HTTP Client — `brrtrouter::http`

**Sesame-IDAM services are may coroutines. They share a single runtime.** Using `reqwest` (built on `tokio`) or direct `may_http` calls duplicates logic and bypasses the shared BRRTRouter fetch layer.

Every outbound HTTP call — including cross-service authz calls, JWKS fetching, and inter-controller calls — must use **`brrtrouter::http`** (re-exported from `sesame_common::http`).

> **Stack:** `sesame_common::http` → `brrtrouter::http` → `may_http` (HTTP) / rustls (HTTPS). Do not import `may_http` directly in service code.

## Canonical usage (wrk-rs pattern)

```rust
use may_http::client::HttpClient;
use http_legacy::{Method, Uri};  // may_http pins http 0.2

let mut client = HttpClient::connect((host, port))?;
client.set_timeout(Some(Duration::from_millis(500)));

// GET shortcut
let mut rsp = client.get(uri)?;

// POST with body + headers
let mut req = client.new_request(Method::POST, uri);
req.headers_mut().insert("content-type", ...);
req.send(body.as_bytes())?;
let mut rsp = client.send_request(req)?;

// Read body (bounded!)
let mut buf = Vec::new();
rsp.by_ref().take(MAX_BYTES).read_to_end(&mut buf)?;
```

**Key properties:** blocking I/O on `may::net::TcpStream` — yields cooperatively inside coroutines; no tokio runtime; one client per connection (no pool yet).

## Current state in sesame-idam (2026-07-06)

| Location | Client | Status |
|----------|--------|--------|
| `sesame-common/src/http.rs` | Re-exports `brrtrouter::http` | ✅ canonical entry point |
| `identity-login-service/.../authz_client.rs` | `sesame_common::fetch_post` | ✅ migrated |
| `common/src/jwks_cache/cache.rs` | `sesame_common::fetch_get` | ✅ migrated |
| Service `Cargo.toml` files | — | ✅ no direct `may_http` / `reqwest` |
| BRRTRouter security providers | `brrtrouter::http` | ✅ migrated (sibling repo) |
| OpenTelemetry (transitive) | `reqwest` rustls | ⚠️ OTEL HTTP exporter only |

## Migration goal: zero reqwest

Transitive `reqwest` from BRRTRouter is **not** the end state. Target:

1. **Service code** — already on `may_http` (done).
2. **BRRTRouter security providers** — replace `reqwest::blocking::Client` in `jwks_bearer`, `remote_api_key`, `spiffe/validation` with `may_http` (cluster-internal HTTP today; HTTPS needs TLS — see gaps).
3. **Extract shared helper** — factor the `authz_client` connect/POST/read pattern into `sesame-common` (or BRRTRouter) so controllers don't duplicate host/port parsing and body limits.
4. **OTEL** — keep gRPC-tonic exporter (already enabled); drop HTTP exporter path if possible to shed OTEL's reqwest dep.

## may_http gaps (fork or upstream contributions)

| Gap | Impact | Workaround today |
|-----|--------|------------------|
| No TLS | Cannot fetch external HTTPS JWKS | BRRTRouter still uses reqwest+rustls |
| No async DNS (`dns.rs` empty) | Manual host/port parse | `parse_host_port()` in authz_client |
| No connection pool / keep-alive reuse | New TCP per request | Acceptable for low-QPS inter-service |
| Header copy on send (wrk-rs TODO) | Extra alloc on hot path | Not yet a bottleneck |

Consider a **microscaler fork** of `may_http` for TLS (rustls over `may::net::TcpStream`) if upstream is inactive.

## BRRTRouter refactor (required for zero reqwest)

Sesame-IDAM service code is migrated; **BRRTRouter is the remaining reqwest consumer** in the hot path. This is a sibling-repo change affecting hauliage and all generated services.

### Direct reqwest call sites (production)

| File | Pattern | Notes |
|------|---------|-------|
| `security/jwks_bearer/mod.rs` | `reqwest::blocking` + `std::thread` background refresh | External HTTPS JWKS |
| `security/spiffe/validation.rs` | Same as JWKS | Duplicated refresh logic |
| `security/remote_api_key.rs` | `reqwest::blocking` on request path | Validates via GET to verify URL |

### Transitive reqwest (dependency tree)

| Crate | Why | Mitigation |
|-------|-----|------------|
| `opentelemetry-otlp` / `opentelemetry-http` | HTTP OTLP exporter | Already use `grpc-tonic`; disable HTTP exporter feature if possible |
| `jsonschema` | Remote schema fetch (optional) | Pin without network features if available |
| `goose` | Dev-only load tests | Accept in dev-deps |

### Proposed BRRTRouter changes

1. **`brrtrouter::http` module** — ✅ **Phase 1 landed (2026-07-06)** in BRRTRouter sibling repo:
   - `src/http/fetch.rs` — `fetch_get`, `fetch_get_text_with_retry`, `HttpFetchOptions`
   - HTTP via `may_http::HttpClient`; HTTPS via rustls on `may::net::TcpStream`
   - Migrated: `remote_api_key.rs`, `jwks_bearer/mod.rs`, `spiffe/validation.rs`
2. **Deduplicate JWKS refresh** — partially done via shared `fetch_get_text_with_retry`
3. **Background refresh: `std::thread` → `may::go!`** — still TODO
4. **Drop direct `reqwest` dep** — still TODO (OTEL/jsonschema transitive remains)
5. **Tests** — dev test helpers still use reqwest blocking

### Sequencing (cross-repo)

```
Phase 1  sesame-common or BRRTRouter::http — shared fetch helper (HTTP only)
Phase 2  BRRTRouter security providers — swap reqwest → may_http (cluster HTTP paths)
Phase 3  may_http fork — rustls TLS layer
Phase 4  BRRTRouter — HTTPS JWKS + drop direct reqwest dep
Phase 5  OTEL — grpc-only export, shed opentelemetry-http reqwest
Phase 6  sesame-idam — pin updated BRRTRouter, verify musl + no openssl-sys
```

> **Resolved (2026-07-06):** `brrtrouter::http` lives in BRRTRouter; sesame consumes via `sesame_common::http`. Remaining BRRTRouter work tracked in [`topic-brrtrouter-refactor-backlog.md`](./topic-brrtrouter-refactor-backlog.md) (BR-5..BR-7).

## TLS Backend: rustls Only

**Do not pull in `openssl-sys` or `native-tls`.** The may ecosystem and our musl container builds require pure-Rust TLS.

| Backend | Status |
|---------|--------|
| `rustls` | **Preferred** — used by `may_http`, BRRTRouter's `reqwest` (`default-features = false, features = ["rustls"]`), and `jsonwebtoken` with `rust_crypto` |
| `openssl-sys` / `native-tls` | **Banned** — breaks `x86_64-unknown-linux-musl` cross-compiles and violates our stack policy |

### Dependency rules

- **Never add `reqwest` directly to service `Cargo.toml`.**
- **`may_http`** is the only permitted outbound HTTP client for service code and (target) BRRTRouter security fetch paths.
- Transitive `reqwest` from BRRTRouter/OTEL is **temporary** — migrate to `may_http` and remove once TLS gap is closed.

### Verification

```bash
cd microservices
cargo tree -p <service> -i openssl-sys   # should report "did not match any packages"
cargo build -p <service> --target x86_64-unknown-linux-musl
```

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

- `may_http` → `git = "https://github.com/rust-may/may_http.git"` — **client + server**, coroutine-native
- `may_minihttp` → server only (microscaler fork for TestClient)
- `wrk-rs/src/main.rs` — reference benchmark using `HttpClient::connect` + `client.get(uri)` in coroutines
- `identity-login-service/.../authz_client.rs` — production inter-service POST pattern
