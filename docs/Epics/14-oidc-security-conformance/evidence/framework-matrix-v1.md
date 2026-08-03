# Framework compatibility matrix — first slice (Epic 14.7)

**Date:** 2026-08-03  
**provider_profile:** `1.0.0`  
**Fixture corpus:** `conformance/oidc-v1` (`fixture_version` 1.1.0)  
**Issuer:** `https://id.sesameidentity.dev.local`

Unmodified libraries consume the same provider + negative access-token fixtures.
This is the thin first cut that unblocks Epic 15 thinking; the full README list
remains the epic exit condition.

| Consumer | Library / artifact | Version | Slice | Positive | Negatives (iss/aud/alg=none/exp) | Notes |
|----------|-------------------|---------|-------|----------|----------------------------------|-------|
| SPA OIDC | Auth.js (`next-auth` / `@auth/core`) | **5.0.0-beta.25** (pinned target) | RP login via discovery | Planned against live issuer | Reject via JWT callback / resource checks | Generic OIDC provider config; no Sesame patches |
| Server RP | Authlib | **1.3.2** | Authorization Code + PKCE | Tooling probe when live | Fixture-driven claim checks | Python; see `tooling` `oidc_framework_matrix` |
| Resource server | Spring Security OAuth2 Resource Server | **6.3.3** (`spring-security-oauth2-jose`) | JWT access-token validation only | N/A (token validate) | Must fail forged set | EdDSA/`at+jwt` via Nimbus |

## Shared negative set

From `conformance/oidc-v1/protocol-cases.json` → `access_token`:

- `wrong_issuer`
- `wrong_audience`
- `alg_none`
- `expired`
- `tenant_mismatch`

Server-side proof for the shared validator: `sesame_common::verify_access_token`
+ BDD `conformance_access_token_forgery_set`.

## Expansion backlog (exit condition)

- Spring Security OAuth2 Login (full browser)
- ASP.NET Core OpenID Connect + JWT Bearer
- OmniAuth OpenID Connect
- PHP OIDC (Laravel adapter target)
- `coreos/go-oidc`

Record exact versions in this file when each row turns green.
