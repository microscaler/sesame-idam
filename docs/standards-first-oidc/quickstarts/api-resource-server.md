# Quickstart: API resource server

## Goal

Validate Sesame access tokens and authorize using the verified principal.

## Validation checklist

1. Fetch JWKS from discovery (`jwks_uri` on `https://id.<zone>`).
2. Require `alg=EdDSA`, `typ=at+jwt`, matching `kid`.
3. Require exact `iss`, intended `aud`, `exp`/`nbf`, `ver`, denylist/version checks.
4. Require `tenant_id` == `sx.tenant`.
5. Map to verified principal
   ([verified-principal-mapping-v1.md](../verified-principal-mapping-v1.md)).

## Policy

- Authorize on `roles` / `permissions` from the principal (from `sx`).
- Treat `organization_id == null` as pre-org; return 403 on org-scoped routes.
- Optional Postgres: Lifeguard contextual transactions (ADR-005), not a
  SesameExecutor wrapper.

## Fixtures

Reject the negative access-token cases in `conformance/oidc-v1` before shipping.
