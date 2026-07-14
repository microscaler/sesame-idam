---
title: HTTP Client Policy
status: verified
updated: 2026-07-13
---

# HTTP Client Policy

## Rule: one coroutine-native outbound HTTP stack

Sesame-IDAM services run on the may coroutine runtime. Service code must use
sesame_common::http, which re-exports the bounded fetch API from
brrtrouter::http.

> **Stack:** sesame_common::http → brrtrouter::http →
> may_minihttp::client for plain HTTP, or rustls over may::net::TcpStream
> for HTTPS.

Do not import may_minihttp::client directly in Sesame service code. Keeping
connection setup, timeouts, response limits, retries, and TLS policy in
BRRTRouter prevents each service from creating a subtly different client.

## Dependency source

The native client is supplied by the Microscaler fork:

~~~toml
may_minihttp = {
  git = "https://github.com/microscaler/may_minihttp.git",
  branch = "integration/microscaler-fork",
  features = ["client"],
}
~~~

As of 2026-07-13, microservices/Cargo.lock resolves that branch at merge
commit 604109761be53ed4f2c1d6fc314b658a7f8a3ba7.

The workspace also patches crates.io may_minihttp dependencies to the same
Git branch. This prevents generated or transitive crates from silently
resolving the stale crates.io release while BRRTRouter uses the fork.

## Canonical usage

~~~rust
use std::time::Duration;

use sesame_common::http::{fetch_get, HttpFetchOptions};

let options = HttpFetchOptions {
    timeout: Duration::from_millis(500),
    max_body_bytes: 256 * 1024,
    extra_headers: Vec::new(),
};

let (status, body) = fetch_get(url, &options)?;
~~~

Use fetch_post for JSON or other request bodies and
fetch_get_text_with_retry for bounded retrying text fetches such as JWKS
refresh. Always set a finite timeout and response-size limit.

## Current state

| Location | Client path | Status |
|----------|-------------|--------|
| idam/common/src/http.rs | Re-exports brrtrouter::http | Canonical service entry point |
| Login authz client | sesame_common::fetch_post | Migrated |
| Shared JWKS cache | sesame_common::fetch_get | Migrated |
| BRRTRouter HTTP fetch/proxy | may_minihttp::client::HttpClient | Native client |
| HTTPS fetch | rustls on may::net::TcpStream | No OpenSSL/native-tls |
| OTEL and test tooling | May retain transitive reqwest | Not a service HTTP client |

may_http is retired from the active Sesame workspace. An excluded historical
generated crate may still contain a stale manifest reference; generated code
must not be hand-edited and will inherit the current BRRTRouter template when
it is regenerated.

## TLS policy

Use rustls only. Do not introduce openssl-sys or native-tls; they break the
workspace pure-Rust TLS policy and complicate musl cross-compilation.

## Banned

- Direct reqwest, hyper, surf, ureq, or isahc in service code.
- Direct may_minihttp::client use outside the shared BRRTRouter HTTP layer.
- tokio::spawn for service background work; use the may runtime.
- Unbounded response reads or requests without finite timeouts.

## Code anchors

- microservices/Cargo.toml
- microservices/idam/common/src/http.rs
- ../BRRTRouter/src/http/fetch.rs (sibling repository)
- ../BRRTRouter/src/http/proxy.rs (sibling repository)

## Gaps / drift

> **Open:** Transitive reqwest remains through observability and development
> dependencies. It is not used by Sesame service request paths, but dependency
> trimming can continue independently of the native-client migration.
