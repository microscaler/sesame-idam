# Authorization microservice

**Role:** Access Management. Register applications (dot-notation slugs), roles, permissions, attributes; assign principals to roles/attributes; evaluate `principal/effective` (for JWT enrichment) and `authorize` (per-request). RBAC + optional ABAC.

**OpenAPI:** Canonical spec in `BRRTRouter/docs/SPIFFY_mTLS/openapi/access-management-openapi.yaml`. A copy or link can live under this repo’s `openapi/idam/authorization/` when implementing.

**Implementation:** When adding Rust implementation, follow RERP layout:

- `gen/` — BRRTRouter-generated crate from OpenAPI.
- `impl/` — Binary crate depending on `gen/`, plus AM DB (lifeguard entities/migrations from IDAM integration doc).

Add to `microservices/Cargo.toml` workspace members: `idam/authorization/gen`, `idam/authorization/impl`.
