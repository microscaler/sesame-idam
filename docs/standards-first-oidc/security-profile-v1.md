# Sesame OIDC Security Profile v1

Status: **normative freeze** (Epic 14.1)  
`provider_profile`: `1.0.0` (must match [`conformance/oidc-v1/manifest.json`](../../conformance/oidc-v1/manifest.json))  
Companion docs: [`provider-profile-v1.md`](./provider-profile-v1.md), [`security-conformance-v1.md`](./security-conformance-v1.md)

This document freezes the security choices consumers and conformance runners
may rely on. Changing a locked rule is a major `provider_profile` bump.

## Locked protocol choices

| Area | Decision |
|------|----------|
| Browser response type | Authorization Code only (`code`) |
| Implicit / hybrid | Not supported |
| PKCE | `S256` required for public clients; preferred for all clients; `plain` rejected |
| Redirect URIs | Exact match against registered URIs (no prefix) |
| Grants | `authorization_code`, `refresh_token` |
| Access token | EdDSA, `typ=at+jwt` (RFC 9068) |
| ID token | EdDSA; RP validates `aud`, `nonce`, `iat`, `azp` where required |
| Refresh | Rotated, client-bound; Redis/server state authoritative; replay fails closed |
| Algorithms | EdDSA allow-list only; `alg=none` and unknown algs fail closed |
| Tenant | Registered client binds tenant before login; validated token binds after; `X-Tenant-ID` never overrides |
| Metadata | Advertises only completed behavior |

## Locked access-token validation

Every credential-minting path and resource server MUST verify, in order:

1. Compact JWT structure and `typ=at+jwt`
2. Signature with an active EdDSA key (`kid` from JWKS / shared keyset)
3. Exact `iss` (= `SESAME_JWT_ISSUER` / BearerAuth.iss)
4. Intended `aud` for the consumer
5. `exp` / `nbf` (with configured leeway)
6. Tenant consistency (top-level and namespaced claims)
7. Token version / denylist policy where enabled

Decoded-but-unverified JWT JSON is never authority (Epic 14.2).

## Fixture and evidence binding

- Corpus: `conformance/oidc-v1/` (`fixture_version` + `provider_profile`)
- Server BDD loads that corpus (no parallel expected-outcome tables)
- External suite target: OpenID Foundation Conformance — Basic / Auth Code + PKCE
- Redacted fields: see `manifest.json` `redacted_fields`

## Compatibility policy

Removing or relaxing a locked rule requires:

1. major `provider_profile` bump;
2. ADR or profile revision note;
3. refreshed Epic 14 release evidence (fixtures, OIDF report, framework matrix).
