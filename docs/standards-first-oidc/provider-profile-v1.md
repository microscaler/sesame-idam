# Sesame OIDC Provider Profile v1

Status: normative  
Profile version: `1.0.0`

## Public origins

- Issuer and keys: `https://id.<zone>`
- Hosted authorization: `https://auth.<zone>/oauth/authorize`
- Token and UserInfo: `https://api.<zone>/oauth/token` and `/oauth/userinfo`

The issuer is exact and immutable within an environment. Consumers discover all
other URLs from `{issuer}/.well-known/openid-configuration`; internal service
names and `/idam/v1` are not consumer configuration.

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
`verified-principal-v1.schema.json`. Roles and permissions come from the `sx`
authorization namespace and are not trusted before token validation.

## Errors and transport

OAuth errors use the standard `error` and optional `error_description` fields.
Token endpoint client failures use `invalid_client`; code, PKCE, and refresh
failures use `invalid_grant`. Temporary state-store failures use
`temporarily_unavailable`. Credentials, tokens, codes, verifiers, and secrets
are redacted.

GET discovery/JWKS responses are cookie-free and cacheable. The API origin
strips inbound `Cookie`, outbound `Set-Cookie`, and caller-provided tenant
headers. Browser CORS uses exact configured origins and never wildcard
credentialing.

## Compatibility policy

Removing or changing a required claim, endpoint, algorithm, or validation rule
is a major profile change. Optional additive claims are minor changes and must
be ignored by consumers that do not understand them. Public API schemas follow
semantic versioning. Deprecations require a migration note and a published
removal window.
