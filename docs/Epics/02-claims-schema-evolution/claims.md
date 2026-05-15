# Epic 2: Claims Schema Evolution

## Summary

Evolve the JWT claim schema from the current flat structure (`user_role`, `user_permissions` array, `email`, `phone_number`) to the namespaced, versioned, bounded schema recommended in the JWT document. Replace flat claims with a collision-resistant custom namespace (`https://sesame-idam.dev/claims`) containing `tenant`, `portal`, `roles`, `permissions`, `entitlements_ref`, `entitlements_hash`, and `risk`.

## Why This Epic Is Needed

The current JWT payload embeds a full permission array and PII fields (email, phone number) in every token. The JWT document identifies these as risks:
- PII in tokens violates the principle of minimal claims (RFC 9068)
- Full permission arrays become stale and bloat tokens
- The current schema lacks versioning for authz snapshot invalidation
- No entitlement reference/hash for large ACL handling
- No tenant claim (despite multi-tenancy being core to Sesame)

## Current State

```json
{
  "sub": "user-uuid",
  "email": "user@example.com",
  "email_verified": true,
  "name": "John Doe",
  "preferred_username": "johnd",
  "user_id": "user-uuid",
  "first_name": "John",
  "last_name": "Doe",
  "org_id": "org-uuid",
  "org_name": "Acme Inc",
  "user_role": "Admin",
  "user_permissions": ["invoices:write", "invoices:read", "users:manage"],
  "mfa_enabled": true,
  "is_platform_admin": false,
  "phone_number": "+141****1234",
  "phone_verified": true,
  "iat": 1705312800,
  "exp": 1705313700
}
```

**Problems:**
- No `tenant` claim (multi-tenancy not in JWT)
- No `ver` / `authz_ver` for snapshot versioning
- No `entitlements_ref` or `entitlements_hash`
- No `scope` claim (RFC 9068)
- No `sid` (session ID)
- No `act` (delegation)
- PII (email, phone) embedded unnecessarily
- No namespaced custom claims structure
- `user_role` is a single string (not compatible with multi-role)

## Stories

- [ ] Story 2.1: Define the new namespaced claim structure
  - Standard claims: `iss`, `sub`, `aud`, `client_id`, `scope`, `exp`, `nbf`, `iat`, `jti`
  - Sesame claims namespace: `https://sesame-idam.dev/claims`
  - Contains: `tenant`, `portal`, `roles`, `permissions`, `entitlements_ref`, `entitlements_hash`, `risk`
  - Version claims: `ver`, `sid`
  - Optional delegation: `act`

- [ ] Story 2.2: Implement the new TokenClaims Rust structs
  - `ActorClaim` for RFC 8693 `act`
  - `SesameAuthzClaims` for namespaced authz data
  - `AccessClaims` as the top-level structure
  - Backward-compatible deserialization during migration

- [ ] Story 2.3: Replace PII fields with references
  - Remove `email` and `phone_number` from access tokens
  - Add `entitlements_ref` and `entitlements_hash` as compact references to the full ACL
  - Consumers can request the full entitlement snapshot via a dedicated endpoint when needed

- [ ] Story 2.4: Add `tenant` to JWT claims
  - Include `tenant` (UUID) in every access token
  - Enables downstream services to validate tenant context without a database call
  - Must be present even for platform-admin tokens

- [ ] Story 2.5: Token size budget enforcement
  - Measure representative token size in bytes
  - Fail build if token exceeds 8KB budget
  - Document token size distribution in observability

## OpenAPI Changes Needed

- `LoginResponse` schema: Add `token_version` field to document the claim version
- No changes needed to request schemas
- Response schemas may need updating to reflect new claim types (for documentation only, not for the token itself)

## Design Doc Changes Needed

- `design-doc.md` section 7 (JWT Enrichment): Replace the current JWT payload example with the new namespaced structure
- `design-doc.md` section 10 (Security Design): Add claims schema evolution section
- Wiki: Create `topics/topic-claims-schema.md` (new) or update `topics/topic-jwt-schema.md`
- Wiki: Update `topics/topic-login-flow.md` to reference new claim structure

## Gaps in the JWT Document

- The proposed JSON example uses `https://sesame-idam.dev/claims` as a namespace key. This is a valid approach per RFC 7519 (registered claims use public URIs, private claims use URIs controlled by the producer). However, this exact URI needs to be reviewed against Sesame's naming conventions.
- The document recommends `entitlements_ref` + `entitlements_hash` but doesn't specify the format of the hash (SHA-256? BLAKE3?) or where the reference points to.
- No migration plan from the current flat schema. How do you handle existing tokens during the transition? How long do you support the old schema?

## Dependencies

- Depends on Epic 1 (JWKS) for validation infrastructure
- Intersects with Epic 5 (versioning) and Epic 6 (delegation)
