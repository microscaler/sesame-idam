# Sesame OIDC Provider Profile v1

Status: normative  
Profile version: `1.0.0`

This document is the **canonical consumer entrypoint** for the Sesame portable
contract. Integration docs that disagree with this profile are stale; prefer
this file and the linked artifacts below.

## Contract map

| Artifact | Role |
|---|---|
| [security-profile-v1.md](./security-profile-v1.md) | Security subset (threat model, redaction, negative cases) |
| [verified-principal-v1.schema.json](./verified-principal-v1.schema.json) | Post-validation principal JSON Schema |
| [verified-principal-mapping-v1.md](./verified-principal-mapping-v1.md) | JWT claim path → principal field |
| [transport-policy-v1.md](./transport-policy-v1.md) | Errors, retries, pagination, rate limits |
| [client-boundaries-v1.md](./client-boundaries-v1.md) | RP / RS / tenant-admin package boundaries |
| [compatibility-v1.md](./compatibility-v1.md) | Versioning and deprecation rules |
| [client-ecosystem-selection-v1.md](./client-ecosystem-selection-v1.md) | Reference SDK selection (Auth.js = Epic 16) |
| [quickstarts/](./quickstarts/) | Browser / BFF / API / worker outlines |
| [Google social credentials runbook](../runbooks/google-social-oauth-credentials.md) | Tenant Google OAuth client + env keys |
| [JWT signing keyset + SOPS runbook](../runbooks/jwt-signing-keyset-sops.md) | Generate Ed25519 keys for JWKS / GitOps |
| [`openapi/idam/tenant-consumer/openapi.yaml`](../../openapi/idam/tenant-consumer/openapi.yaml) | Sole public SDK OpenAPI |
| [`conformance/oidc-v1/`](../../conformance/oidc-v1/) | Versioned protocol fixtures + checksum |

## Public origins

- Issuer and keys: `https://id.<zone>`
- Hosted authorization: `https://auth.<zone>/oauth/authorize`
- Token and UserInfo: `https://api.<zone>/oauth/token` and `/oauth/userinfo`
- Tenant consumer API: `https://api.<zone>/idam/v1`

The issuer is exact and immutable within an environment. Consumers discover all
other OIDC URLs from `{issuer}/.well-known/openid-configuration`; internal service
names are not consumer configuration.

## Protocol

- Authorization Code is the only response type.
- PKCE `S256`, state, nonce, and an exact registered redirect URI are required.
- Public clients use `token_endpoint_auth_method=none`.
- Confidential clients use their registered `client_secret_basic` or
  `client_secret_post` method. Secrets must not be placed in URLs or logs.
- Supported grants are `authorization_code` and `refresh_token`.
- Codes are opaque, expire after 60 seconds, and are atomically single-use.
- Refresh tokens are client-bound, rotated on every use, and replay-protected.
- Access tokens use EdDSA and `typ=at+jwt`; ID tokens use EdDSA.
- UserInfo is bearer protected and always returns the same `sub` as the tokens.
- Implicit, hybrid, password, device, dynamic registration, and public token
  exchange are not part of profile v1.

## Access-token claims

Aligned with runtime `AccessClaims`. Authorization claims live under the
namespace `https://sesame-idam.dev/claims` (documented as `sx.*` below).

| Claim | Required | Notes |
|---|---|---|
| `iss` | yes | Exact environment issuer |
| `sub` | yes | Stable subject; equals `user_id` |
| `aud` | yes | Audience array |
| `client_id` | yes | Registered OAuth client |
| `scope` | yes | Space-delimited scopes |
| `exp` / `nbf` / `iat` | yes | Unix seconds |
| `jti` | yes | Unique token id |
| `ver` | yes | Token version (≥ 1) |
| `sid` | yes | Session id |
| `tenant_id` | yes | Tenant partition |
| `user_id` | yes | Same value as `sub` |
| `user_type` | yes | e.g. `customer`, `platform` |
| `org_id` | no | Active org; **omit or null** until user creates/joins an org |
| `sx.tenant` | yes | Must equal `tenant_id` |
| `sx.portal` | yes | Portal / application surface name |
| `sx.roles` | yes | Array (may be empty) |
| `sx.permissions` | yes | Coarse hints (may be empty) |
| `sx.entitlements_ref` | no | Cache key for full ACL |
| `sx.entitlements_hash` | no | Integrity hash for cached ACL |
| `sx.risk` | no | Risk band when elevated |
| `act` | no | RFC 8693 actor |
| `cnf` | no | DPoP confirmation |

Header: `alg=EdDSA`, `typ=at+jwt`, `kid` present.

## ID-token claims

Minted by the authorization server for the OpenID `openid` scope:

| Claim | Required | Notes |
|---|---|---|
| `iss` | yes | Same issuer as access token |
| `sub` | yes | Same subject as access token |
| `aud` | yes | Client id (string) |
| `azp` | yes | Authorized party (= client id) |
| `exp` / `iat` | yes | Unix seconds |
| `auth_time` | yes | Authentication time |
| `nonce` | yes | Echo of authorize request nonce |

Header: `alg=EdDSA`. ID tokens are not access tokens (`typ` is not `at+jwt`).

## Validation

Resource servers must validate signature, `alg=EdDSA`, key use, `kid`, exact
issuer, intended audience, `typ=at+jwt`, `exp`, `nbf`, token version, denylist,
and the consistency of top-level and namespaced Sesame claims. Relying parties
must additionally validate ID-token audience, `azp`, nonce, `iat`, and
`auth_time`. Decoded but unverified JWT JSON is never authority.

## Tenant and principal semantics

The registered client determines tenant and application before login. A
validated access token determines them afterward. `X-Tenant-ID` cannot select
or override tenancy. `org_id` is optional because a newly registered user may
not yet belong to an organization.

After validation, adapters normalize claims according to
`verified-principal-v1.schema.json` using
[verified-principal-mapping-v1.md](./verified-principal-mapping-v1.md). Roles and
permissions come from the `sx` authorization namespace and are not trusted before
token validation.

## Errors and transport

See [transport-policy-v1.md](./transport-policy-v1.md). OAuth errors use the
standard `error` and optional `error_description` fields. Token endpoint client
failures use `invalid_client`; code, PKCE, and refresh failures use
`invalid_grant`. Temporary state-store failures use `temporarily_unavailable`.
Credentials, tokens, codes, verifiers, and secrets are redacted.

GET discovery/JWKS responses are cookie-free and cacheable. The API origin
strips inbound `Cookie`, outbound `Set-Cookie`, and caller-provided tenant
headers. Browser CORS uses exact configured origins and never wildcard
credentialing.

## Compatibility policy

See [compatibility-v1.md](./compatibility-v1.md). Removing or changing a
required claim, endpoint, algorithm, or validation rule is a major profile
change. Optional additive claims are minor changes and must be ignored by
consumers that do not understand them.
