![Sesame Logo](./ui/images/logo.png)

# Sesame-IDAM

Identity and Access Management (IDAM) as **microservices** for the Microscaler platform (BRRTRouter, RERP, PriceWhisperer, etc.). This repo is **not** a standalone SaaS IDAM product; it provides the **Authentication** and **Authorization** components that plug into our microservice architecture — or any open-source team that chooses to build on top of BRRTRouter.

## Features

| Function | Description | API Spec Complete | BDD Tests created | Implemented |
|----------|-------------|-------------------|-------------------|-------------|
| login | | | | |
| User registration | | | | |
| Password hashing | | | | |
| Password reset | | | | |
| Email verification | | | | |
| JWT authentication | | | | |
| User management | | | | |
| Role-based access control | | | | |
| Rate limiting | | | | |
| Organisation management | | | | |
| Waitlist | | | | |
| API Key Authentication | | | | |
| Security | | | | |
| User properties | | | | |
| Roles and Permissions (RBAC) | | | | |
| Advanced RBAC | | | | |
| User impersonation | | | | |
| User Management (backend admin panel or each organisations administrator) | | | | |
| Metrics & user insights | | | | |
| Enterprise SSO (SAML) | | | | |
| MFA Enforcement | | | | |
| SCIM (System for Cross Domain Identity Management) | | | | |
| Restricted login methods | | | | |
| API Key rate limiting | | | | |
| Audit logs | | | | |

## Layout (RERP-style)

The repo is structured like [RERP](https://github.com/microscaler/rerp) with a **microservices** directory:

```
microservices/
  Cargo.toml          # Workspace (members added when implementing)
  idam/
    README.md         # IDAM domain overview
    authentication/   # Identity & auth microservice (login, sessions, JWTs, …)
      README.md
    authorization/    # Access Management microservice (roles, permissions, authorize, …)
      README.md
openapi/
  idam/               # OpenAPI specs (canonical: BRRTRouter/docs/SPIFFY_mTLS/openapi/)
    authentication/
    authorization/
```

**Identified microservices:**

| Service | Purpose |
|---------|--------|
| **Authentication** | Identity, login, refresh, logout, token exchange, register, sessions, JWKS/OIDC. Aligns with [Generic Identity Service](https://github.com/microscaler/BRRTRouter/blob/main/docs/SPIFFY_mTLS/Generic_Identity_Service_IDAM_Design.md). |
| **Authorization** | Access Management: applications, roles, permissions, principal assignments, `principal/effective`, `authorize`. Aligns with [Generic Access Management Service](https://github.com/microscaler/BRRTRouter/blob/main/docs/SPIFFY_mTLS/Generic_Access_Management_Service_Design.md). |

Further IDAM components may be added under `microservices/idam/` as we identify them.

## Reference: original OpenAPI backup

The original (pre-pivot) OpenAPI spec is backed up at **`specs/backup/openapi-original-sesame.yaml`** for reference. Use it when implementing the Features above so the new Authentication and Authorization microservices provide that rich functionality (and more). See `specs/backup/README.md` for details.

## Tooling and guard rails (same as RERP)

Sesame-IDAM uses a **tooling/** Python package with the same strict **ruff** and **pre-commit** guard rails as [RERP](https://github.com/microscaler/rerp): ruff (E, F, W, B, C4, UP, SIM, I, PTH, RUF, …), format-check, pytest, and forbidden empty print statements. Run **`just init`** once, then **`just qa`** before commit; **`just install-hooks`** to install pre-commit. See **AGENTS.md** and **tooling/README.md**.

## Development environment (same DX as RERP)

Sesame-IDAM uses the same **Kind + Tilt** DX as [RERP](https://github.com/microscaler/rerp):

- **`just init`** — Create tooling venv (required once before `just dev-up`).
- **`just dev-up`** — Create Kind cluster `sesame-idam`, local registry (localhost:5001), namespace, then start Tilt (UI on port 10351).
- **`just dev-down`** — Stop Tilt and delete the Kind cluster (registry left running).
- **`just dev-down-full`** — Same as dev-down and remove the local registry.
- **`just up`** / **`just down`** — Start/stop Tilt when the cluster already exists.
- **`just status`** — Show cluster and pods/services in `sesame-idam` namespace.

**Kind cluster:** Same port mappings as RERP — PostgreSQL (host 5433), Redis (6379), Prometheus (9091), Grafana (3002). Data dir `/tmp/sesame-idam-data` is mounted into the node for PVs.

**Data components (same as RERP):** PostgreSQL (Service `postgresql`, user/db `sesame_idam`) and Redis (Service `redis`) are deployed by Tilt from `k8s/data/`. Helm app config includes `database` and `redis` (host `postgresql`, `redis`) so IDAM services can connect. Run **`just port-forward`** to forward postgres (5432) and redis (6379) to localhost.

**Containerised packaging:** `docker/microservices/Dockerfile.template` and **Helm** chart `helm/sesame-idam-microservice/` with values for authentication (8001) and authorization (8002). Microservice images are built and deployed by Tilt once gen+impl crates exist.

## Build and deploy (BRRTRouter tooling)

This repo consumes **shared BRRTRouter tooling** for codegen and API try-out (same idea as RERP). From the repo root, with BRRTRouter available (sibling `../BRRTRouter` or `BRRTRouter_DIR` set):

- **`just gen`** — Regenerate both Authentication and Authorization gen crates from the OpenAPI specs in `openapi/idam/`.
- **`just gen-auth`** / **`just gen-authorization`** — Regenerate one microservice’s gen crate.
- **`just lint-openapi`** — Lint both specs (brrtrouter-gen).
- **`just serve-auth`** / **`just serve-authorization`** — Run echo servers for local try-out.

See **AGENTS.md** for the full command table and layout.

## Status

- **Rust:** All previous Rust code has been removed. Implementations will be added as `gen/` + `impl/` crates per microservice (BRRTRouter codegen + lifeguard for persistence), following the RERP pattern.
- **OpenAPI:** Two specs in `openapi/idam/authentication/openapi.yaml` and `openapi/idam/authorization/openapi.yaml` (derived from BRRTRouter canonical). Use `just gen` to generate code; `just sync-specs-from-brrtrouter` to refresh from canonical.
- **Audit:** See [Sesame_IDAM_Audit_and_Transformation_Analysis.md](https://github.com/microscaler/BRRTRouter/blob/main/docs/SPIFFY_mTLS/Sesame_IDAM_Audit_and_Transformation_Analysis.md) in the BRRTRouter repo for the full pivot rationale and roadmap.

## Archive

Pre-pivot state (single “Sesame” SaaS IDAM crate with Sea-ORM) is preserved on branch `archive/saas-idam-pre-microservice-pivot`, tag `archive/saas-idam-2025-02-02`, and repo **git@github.com:microscaler/sesame-idam-archived.git**.
