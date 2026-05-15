# Epic 8: Security Hardening

## Summary

Implement the security measures recommended by the JWT document: algorithm allow-listing (RFC 8725), sender constraint (DPoP or mTLS for high-risk channels), token substitution prevention, and hardened claim validation. Replace the current HS256 development fallback with a production-ready JWKS-based validation that enforces `typ`, `iss`, `aud`, and rejects `alg: none`.

## Why This Epic Is Needed

The JWT document identifies four security trade-offs that must be engineered explicitly:
1. Stale permissions (addressed by token versioning, Epic 5)
2. Token substitution and privilege confusion (RFC 8725 compliance)
3. Token theft and replay (DPoP, sender constraint)
4. Shared-secret blast radius (addressed by asymmetric signing, Epic 1)

Additionally, the JWT document flags that the current codebase disables signature validation to extract `jti` before full validation -- "acceptable only as a pre-validation optimisation for denylist lookup. It must never become a trust decision path by itself."

## Current State

- JWT code signs with HS256 from shared `JWT_SECRET`
- Generated runtime contains development fallback `BearerJwtProvider` using simple signature string
- `extract_jti` helper disables signature validation to extract jti before full validation
- No algorithm allow-listing
- No `typ` enforcement (`at+jwt`)
- No sender constraint (DPoP)
- No algorithm confusion protection

## Stories

- [ ] Story 8.1: Implement algorithm allow-listing (RFC 8725)
  - Validate algorithm from an explicit allow-list (ES256, EdDSA, RS256)
  - Reject `alg: none` explicitly (RFC 8725 Section 3.5)
  - Validate `typ = at+jwt` (RFC 9068 Section 2.1)
  - Reject tokens with unrecognized or missing `typ`

- [ ] Story 8.2: Implement issuer and audience validation
  - Validate `iss` matches expected issuer
  - Validate `aud` contains expected audience (supports multiple audiences per RFC 7519)
  - Reject tokens with wrong `iss` or missing `aud`

- [ ] Story 8.3: Implement clock-skew handling
  - Validate `nbf <= now + 60 seconds` (allow 60s skew)
  - Validate `exp > now - 60 seconds` (allow 60s skew)
  - Document skew handling in JWT validation section of design doc

- [ ] Story 8.4: Fix extract_jti security issue
  - Never use unvalidated jti as a trust decision
  - Extract jti only after full signature validation
  - If pre-validation jti lookup is needed for denylist, validate signature separately first
  - Add a comment: "WARNING: this is not a trust path"

- [ ] Story 8.5: Implement DPoP (Proof-of-Possession) for high-risk channels
  - RFC 9449 DPoP binds tokens to a specific client
  - Client must present a DPoP proof with each request
  - Server validates DPoP signature matches the access token's `htm` (HTTP method) and `htu` (HTTP URI)
  - Required for: API key validation, token exchange, impersonation, admin actions

- [ ] Story 8.6: Implement security regression tests
  - `alg: none` attack vectors
  - Wrong issuer, wrong audience, wrong token type
  - Expired token, replayed refresh token, delegated-token misuse
  - Malformed JWT fuzzing
  - Oversized claims (header budget tests)

## OpenAPI Changes Needed

- No OpenAPI changes needed (all security is at validation layer)
- Document security requirements (algorithm allow-list, DPoP) in endpoint descriptions

## Design Doc Changes Needed

- `design-doc.md`: Update "Security Design" section with RFC 8725/9068/9449 compliance
- `design-doc.md`: Add algorithm allow-list table
- `design-doc.md`: Add DPoP integration section
- Wiki: Create `topics/topic-security-hardening.md` (new)
- Wiki: Update `topics/topic-jwt-schema.md` with validation requirements

## Gaps in the JWT Document

- DPoP adds client-side complexity (key generation, proof signing). Does the frontend SDK need DPoP support, or only backend-to-backend?
- The document recommends `act` claim validation but doesn't specify how to validate the actor's right to act on behalf of the subject. Does the admin need a specific permission to impersonate?
- No mention of JWT claims encryption (JWE). All claims are signed but not encrypted -- if a token is intercepted, claims are readable. Is this acceptable for Sesame's threat model?

## Dependencies

- Depends on Epic 1 (JWKS) for asymmetric key infrastructure
- Blocks nothing but is required before any production deployment
- Story 8.5 (DPoP) is optional for v1 -- can be deferred to post-launch
