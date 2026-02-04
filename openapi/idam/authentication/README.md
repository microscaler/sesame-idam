# Authentication (Identity) API

**Spec:** `openapi.yaml` in this directory (derived from BRRTRouter canonical `docs/SPIFFY_mTLS/openapi/identity-openapi.yaml`).

Covers: auth (login, refresh, logout, token exchange, register), identity (email/phone lookup, users/me), organisations and tenants, discovery (OIDC, JWKS). No PII in URIs.

- **Regenerate from this spec:** `just gen-auth` (requires BRRTRouter at `BRRTRouter_DIR` or `../BRRTRouter`).
- **Sync from canonical:** `just sync-specs-from-brrtrouter` then restore the Sesame-IDAM header comment in `openapi.yaml`.
