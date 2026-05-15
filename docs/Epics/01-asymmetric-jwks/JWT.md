# Epic 1: Asymmetric JWT & JWKS

## Summary

Move from symmetric HS256 signing (shared secret across all validators) to asymmetric signing (ES256 or EdDSA) with JWKS key publication and per-service public-key validation. This eliminates shared-secret blast radius and enables stateless JWT validation without every service holding the signing key.

## Why This Epic Is Needed

The JWT document flags "shared-secret blast radius" as one of four security trade-offs. Currently all 6 services that validate JWTs would need the same `JWT_SECRET`. In a multi-service deploy, that means every validator is also a potential signer. Asymmetric signing is operationally safer and fits the repo's existing runtime support (`JwksBearerProvider` is already in the generated runtime).

## Current State

- JWT claims module signs with HS256 from a shared `JWT_SECRET`
- Generated runtime contains `JwksBearerProvider` support (but not wired for authz)
- Generated runtime contains a development fallback `BearerJwtProvider` using simple signature string
- No JWKS publication endpoint in the current spec
- No asymmetric signing key generation or rotation

## Stories

- [ ] Story 1.1: Generate and rotate asymmetric signing keys (ES256 or EdDSA)
  - Generate RSA or Ed25519 key pair at service bootstrap
  - Rotate keys on schedule or on-demand with overlapping validity windows
  - Store private key in memory only, never on disk

- [ ] Story 1.2: Implement JWKS publication endpoint (`/.well-known/jwks.json`)
  - Serve the public key set in standard JWKS format (RFC 7517)
  - Include `kid` for key identification
  - Cache and serve near-static response (NEGLIGIBLE cost per topology design)

- [ ] Story 1.3: Wire all services to validate JWTs via JWKS
  - Each service fetches JWKS from idam on startup, caches for 5 minutes
  - Validates `typ = at+jwt`, algorithm allow-list, `iss`, `aud`, `exp`
  - Reject `alg: none` (RFC 8725 compliance)

- [ ] Story 1.4: Deprecate HS256 signing path
  - Once all validators confirmed working, remove HS256 code path
  - Keep HS256 as a debug-only mode behind feature flag

## OpenAPI Changes Needed

- `/.well-known/jwks.json` endpoint needs to be added to identity-session-service spec (or the combined identity spec)
- All security schemes that reference API key should also support `bearer` + JWKS

## Design Doc Changes Needed

- `design-doc.md` section 10 (Security Design): Add JWKS section, asymmetric signing details
- `sesame-idam-complete.md` section 7 (JWT Enrichment): Note algorithm migration
- Wiki: Update `topics/topic-jwt-schema.md` status to reflect asymetric signing
- Wiki: Update `topics/topic-authorization-flow.md` to note JWKS cache TTL

## Gaps in the JWT Document

- Does not specify which asymmetric algorithm to choose (ES256 vs EdDSA vs RS256). EdDSA has best security/performance; ES256 has widest library support. RS256 adds key size but slower.
- Does not address JWKS cache invalidation strategy (what happens when keys rotate while consumers have stale JWKS).
- Does not address the transition period: how long to support HS256 for existing tokens, how to communicate key rotation to consumers.

## Dependencies

- Blocks Stories 2.1, 4.1, 5.1 (JWT validation foundation for everything else)
- Requires the identity-session-service to already serve JWKS (already in topology spec)
