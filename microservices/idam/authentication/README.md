# Authentication microservice

**Role:** Identity and authentication. Login, refresh, logout, token exchange (RFC 8693), registration, sessions, JWKS and OIDC discovery. Issues user and service JWTs; no PII in URIs; tenant- and organisation-scoped.

**OpenAPI:** Canonical spec in `BRRTRouter/docs/SPIFFY_mTLS/openapi/identity-openapi.yaml`. A copy or link can live under this repo’s `openapi/idam/authentication/` when implementing.

**Implementation:** When adding Rust implementation, follow RERP layout:

- `gen/` — BRRTRouter-generated crate from OpenAPI (controllers, handlers, registry).
- `impl/` — Binary crate depending on `gen/`, plus persistence (lifeguard) and any Identity-specific logic.

Add to `microservices/Cargo.toml` workspace members: `idam/authentication/gen`, `idam/authentication/impl`.
