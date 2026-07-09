---
title: Scaling Profiles
status: partially-verified
updated: 2026-01-22
sources: [design-doc.md, service-topology-design.md]
---

# Scaling Profiles

## Per-Service Scaling

### identity-login-service
- **Compute:** Password hashing is bottleneck (CPU-bound). Consider Argon2id with tuned params.
- **Storage:** PostgreSQL (user data), Redis (session cache, OTP tokens)
- **Horizontal:** Stateless after DB connect. Redis session cache eliminates cross-node state.
- **Vertical:** Password hashing limits per-instance throughput.

### identity-session-service
- **Compute:** Token refresh is DB lookup + JWT sign. OIDC/JWKS are static (negligible).
- **Storage:** PostgreSQL (refresh tokens), Redis (session cache, JWKS cache)
- **Horizontal:** Stateless. Pure session cache + DB.
- **Vertical:** Cache hit ratio should exceed 99% for active users.

### identity-user-mgmt-service
- **Compute:** DB reads/writes. Email/phone sends add latency.
- **Storage:** PostgreSQL (users, MFA secrets, social tokens)
- **Horizontal:** Stateful via DB. No cache pressure.
- **Vertical:** No special constraints.

### authz-core
- **Compute:** Role evaluation is fast (<1ms with cache). `principal/effective` at login is heavy.
- **Storage:** PostgreSQL (role/perm definitions), Redis (per-principal perm cache, 30s TTL)
- **Horizontal:** Can shard by `org_id` (permissions are org-scoped).
- **Vertical:** Limited by Redis latency, not compute.

### api-keys
- **Compute:** Simple SHA-256 hash comparison. Trivial CPU.
- **Storage:** PostgreSQL (key metadata)
- **Horizontal:** Stateless hash lookup. Can shard by `user_id` or `org_id` suffix.
- **Vertical:** One core handles tens of thousands validations/sec.

### org-mgmt
- **Compute:** CRUD operations. SSO involves external HTTP calls (IdP metadata, SAML XML).
- **Storage:** PostgreSQL (org data). No cache needed.
- **Horizontal:** Single instance handles all traffic. Auto-scale to zero if needed.
- **Vertical:** No constraints.

## Cache Strategies

| Service | Cache | TTL | Purpose |
|---------|-------|-----|---------|
| identity-session-service | Redis | Session-based | Session lookups |
| authz-core | Redis | 30s | Permission resolution |
| api-keys | None needed | — | Hash lookup is fast enough |

## Code Anchors

- `Tiltfile` — Tilt deployment config (separate Tilt targets per service)
- `helm/sesame-idam-microservice/` — Helm values for per-service deployment

## Gaps / Drift

> **Open:** Verify actual cache TTLs and strategies against implementation.
