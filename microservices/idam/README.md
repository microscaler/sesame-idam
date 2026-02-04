# IDAM domain

Identity and Access Management (IDAM) microservices. Layout follows RERP-style: each service can have `gen/` (BRRTRouter-generated API crate) and `impl/` (implementation binary).

## Microservices

| Service | Description |
|---------|-------------|
| **authentication** | Identity and auth: login, refresh, logout, token exchange, register, sessions, JWKS/OIDC discovery. Aligns with [Generic Identity Service](https://github.com/microscaler/BRRTRouter/blob/main/docs/SPIFFY_mTLS/Generic_Identity_Service_IDAM_Design.md) and `identity-openapi.yaml`. |
| **authorization** | Access Management: applications, roles, permissions, principal assignments, `principal/effective`, `authorize`. Aligns with [Generic Access Management Service](https://github.com/microscaler/BRRTRouter/blob/main/docs/SPIFFY_mTLS/Generic_Access_Management_Service_Design.md) and `access-management-openapi.yaml`. |

Further IDAM components (e.g. discovery, session store) may be added under `idam/` as needed.

## OpenAPI

Specs are under repo root `openapi/idam/` (see `openapi/idam/authentication/`, `openapi/idam/authorization/`). Canonical sources: `BRRTRouter/docs/SPIFFY_mTLS/openapi/`.
