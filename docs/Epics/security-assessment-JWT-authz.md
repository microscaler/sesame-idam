# Security Assessment: JWT Authorisation Load Mitigation Epics

Date: 2026-05-16
Assessor: Security Expert (JWT, OAuth 2.1, API Security)
Scope: docs/Epics/INDEX.md + 9 epics, 37 stories
Reference: docs/Sesame-idam_authorisation_load_mitigation_with_JWT_claims.md

## Executive Summary

The 9-epic, 37-story plan faithfully follows the JWT document's architectural guidance and is well-structured. However, there are **6 critical gaps**, **6 high-priority issues**, **5 medium-priority issues**, and **4 low-priority items** that must be addressed before implementation.

**Total issues: 21**

| Severity | Count | Key Stories Affected |
|----------|-------|---------------------|
| Critical | 6 | 1.1, 1.3, 3.3, 6.1, 8.2 |
| High | 6 | 2.3, 4.3, 1.2 |
| Medium | 5 | 5.2, 1.1, 6.1, 6.3 |
| Low | 4 | INDEX, 4.x, 3.5, 3.1 |

## CRITICAL FINDINGS

### F-001: Algorithm Choice is Inverted (Epic 1)

**Story:** 1.1
**Finding:** ES256 chosen as default. EdDSA/Ed25519 has mathematically stronger security margins, is immune to timing side-channels by design, and produces shorter signatures (64 bytes vs ~71 bytes for ES256). For a JWT system validating signatures on every request, EdDSA is the correct default. ES256 should be co-default for interoperability.

**Impact:** Weaker crypto default; longer tokens increase NGINX header pressure.

**Fix:** Change Story 1.1 to EdDSA as default, ES256 as co-default.

### F-002: `extract_jti` Helper Disables Signature Validation (All Validation Stories)

**Stories:** 1.3, 4.2, 4.3, 5.3
**Finding:** The current repo's `extract_jti` helper disables signature validation to extract `jti` before full validation. The JWT document warns: "must never become a trust decision path by itself." None of the validation pipeline stories address removing this pattern or enforcing validation order.

**Impact:** If `extract_jti` is used in production, an attacker could supply a valid jti from a forged token and bypass signature validation.

**Fix:** Add explicit pipeline ordering requirement to Stories 1.3 and 4.2. Remove or deprecate `extract_jti`.

### F-003: Token Exchange Missing Required RFC 8693 Claims (Epic 6)

**Story:** 6.1
**Finding:** RFC 8693 requires `iss`, `aud`, `iat`, `exp`, `sub`, `jti` in exchanged tokens. Story 6.1's validation pipeline does not explicitly require `aud` or `iss` in the exchanged token. The TokenExchangeResponse schema does not include `aud` or `iss`.

**Impact:** Non-compliant tokens vulnerable to audience confusion attacks.

**Fix:** Add `aud` and `iss` to TokenExchangeResponse schema and validation pipeline in Story 6.1.

### F-004: No DPoP / Modern Token Binding (Epic 8)

**Story:** 8.2
**Finding:** TLS-SNI token binding only works when the client extracts SNI from the TLS handshake (server-side). The JWT document explicitly references DPoP (RFC 9449) as "the standards-track mechanism that binds access and refresh tokens to a proof-of-possession key, and the spec positions it specifically as an alternative where mTLS token binding is not practical." DPoP works at the application layer and protects against proxy-based token theft even when TLS terminates at a load balancer.

**Impact:** Token binding only protects in narrow TLS-termination scenarios; browser-based clients have no protection.

**Fix:** Replace Story 8.2 with DPoP implementation. TLS-SNI can be added as a secondary mechanism for internal services.

### F-005: Refresh Token Reuse Does Not Invalidate All Sessions (Story 3.1)

**Story:** 3.1
**Finding:** When reuse is detected, the story revokes the compromised family. However, Story 3.5's `user:{user_id}:families` registry and Story 3.2's family isolation mean only ONE session is revoked. The legitimate user's other sessions remain valid. This is actually the correct behavior (Story 3.2 explicitly documents this). However, Story 3.1 does NOT mention that the user must be notified across all sessions.

**Impact:** User may not know their account is compromised on other devices.

**Fix:** Add cross-session notification requirement to Story 3.1 (push notification, email, or in-app signal).

### F-006: Step-Up MFA Does Not Rotate Refresh Token (Epic 6)

**Story:** 6.3
**Finding:** Step-up MFA returns a new access token with `mfa_verified=true` and a new refresh token. The old refresh token remains valid in Redis. An attacker who stole the refresh token before the step-up MFA could use it to obtain a new token with `mfa_verified=true`.

**Impact:** Step-up MFA is defeated by pre-existing stolen refresh tokens.

**Fix:** Invalidate the old refresh token family on successful step-up MFA (add to Story 6.3 acceptance criteria).

## HIGH-PRIORITY FINDINGS

### F-007: No Consumer-Side Entitlements Hash Verification (Epic 2/4)

**Stories:** 2.3, 4.3, 4.4
**Finding:** The `entitlements_hash` is included in the JWT but no story implements consumer-side verification of this hash before trusting a cached entitlements snapshot from Redis. Without hash verification, a compromised Redis cache could serve a modified entitlements snapshot and consumers would accept it.

**Impact:** Tenant bleed if Redis is compromised — attacker injects modified ACL snapshot.

**Fix:** Add consumer-side hash verification step to Story 4.3 (fallback cache handler).

### F-008: Fallback Cache Key Missing `tenant_id` (Epic 4)

**Story:** 4.3
**Finding:** The fallback cache key is `blake3(subject + org_id + action + resource_id)`. It does NOT include `tenant_id`. In a multi-tenant system where `X-Tenant-ID` header is the isolation boundary, this creates a theoretical tenant bleed if two different tenant-scoped requests happen to share the same subject/org/action/resource combination.

**Impact:** Potential tenant data bleed via cache key collision.

**Fix:** Add `tenant_id` to the fallback cache key in Story 4.3.

### F-009: JWKS Endpoint Has No Rate Limiting (Epic 1)

**Story:** 1.2
**Finding:** The JWKS endpoint `/.well-known/jwks.json` is public (correct per RFC 7517) but has no rate limiting. With 6 services fetching every 5 minutes plus external OAuth consumers, this endpoint could receive hundreds of requests/second from an attacker.

**Impact:** DoS on key distribution; potential CPU exhaustion from JSON serialization.

**Fix:** Add rate limiting (100 req/s) to Story 1.2. Document rate limit policy in OpenAPI.

### F-010: Token TTLs Too Short for Admin Roles (Story 3.3)

**Story:** 3.3
**Finding:** 1-3 minute admin tokens create operational burden without meaningful security gain over 5-minute tokens. Admins already have elevated risk through other controls. Step-up MFA (Epic 6) is the correct mechanism for high-consequence admin actions, not shorter tokens.

**Impact:** Operational friction; more refresh operations increase Redis load; false sense of security (admin can still act for 3 minutes without MFA).

**Fix:** Set admin TTL to 5 minutes (same as normal). Document that step-up MFA provides the real security boundary for admin actions.

### F-011: No Key Health Monitoring (Epic 1)

**Story:** 1.1
**Finding:** Key rotation happens at 30-day intervals. If rotation fails (new key generated but service crashes before JWKS update), existing tokens become un-verifiable. There is no monitoring for "keys in JWKS vs keys expected."

**Impact:** Silent key rotation failure; service outage when tokens can no longer be validated.

**Fix:** Add health check endpoint to Story 1.2 that reports key count and ages. Alert if only 1 key present (should be 2 during overlap window).

### F-012: Token Exchange Does Not Validate Audience (Story 6.1)

**Story:** 6.1
**Finding:** The token exchange creates a new access token with merged scopes but does not validate or set the `aud` claim. The new token's `aud` should include the audience of the original token AND the audience of the actor token.

**Impact:** Cross-service token misuse; token issued for service A could be accepted by service B.

**Fix:** Add audience merging/validation to Story 6.1.

## MEDIUM-PRIORITY FINDINGS

### F-013: Version Cache TTL Creates Stale-State Window (Story 5.2)

**Story:** 5.2
**Finding:** Version cache TTL is 15 seconds (subject) and 60 seconds (tenant). After TTL expires, the cache is empty and validators skip version checks (fail-open). Worst-case stale window: 60s (cache expired) + 5min (token TTL) = ~5.5 minutes. This contradicts the goal of near-real-time revocation.

**Impact:** Admin permission changes could be effective up to 5.5 minutes after revocation.

**Fix:** Increase tenant version TTL to 5 minutes (matching token TTL) or use push invalidation (Story 5.4) to keep cache populated.

### F-014: Endpoint Classification Is Approximate (Story 4.1)

**Story:** 4.1
**Finding:** The story acknowledges "approx. 40", "approx. 50", "approx. 43" endpoints but does not provide a definitive classification of all 133 endpoints. The document references "133 endpoints" but AGENTS.md says "119 endpoints" and the service topology says "119 endpoints."

**Impact:** Incomplete classification leads to misclassified routes (high-risk route in jwt-only = security breach).

**Fix:** Add audit phase to Story 4.1 to reconcile endpoint count and produce definitive classification.

### F-015: Refresh Token Binding Not Implemented (Story 3.1)

**Story:** 3.1
**Finding:** Token binding (Epic 8) applies to access tokens but refresh tokens are not bound. A stolen refresh token can be replayed from any client. Refresh token binding (DPoP or sender-constrained) is essential for family-based revocation to work properly.

**Impact:** Stolen refresh tokens work from any device until reuse is detected.

**Fix:** Add refresh token binding to Story 3.1. Document in Story 8.2.

### F-016: Step-Up MFA Implementation Missing MFA Types Detail (Story 6.3)

**Story:** 6.3
**Finding:** Story 6.3 mentions TOTP, WebAuthn, SMS as MFA types but does not address security ordering (SMS < TOTP < WebAuthn) or configuration of acceptable MFA types per action type. SMS is significantly weaker than TOTP or WebAuthn.

**Impact:** Weak MFA may be accepted for high-consequence actions.

**Fix:** Add MFA strength requirements to Story 6.3 acceptance criteria. SMS should require explicit configuration flag.

### F-017: No Dead Token Sweep Documentation (Story 3.5)

**Story:** 3.5
**Finding:** Stories 3.1 and 3.2 manage refresh token families but do not address periodic cleanup of expired refresh tokens. Redis TTL handles key expiration, but the `family:{family_id}` sets and `user:{user_id}:families` sets do not have matching TTLs, causing unbounded Redis growth.

**Impact:** Redis memory growth over time.

**Fix:** Add TTL to `user:{user_id}:families` matching the refresh token TTL in Story 3.5.

## LOW-PRIORITY FINDINGS

### F-018: Endpoint Count Inconsistency Across Documents

**Finding:** INDEX.md says 133 endpoints. AGENTS.md says 119. Service topology says 119. The epics reference 133. Factual inconsistency must be reconciled before Story 4.1 classifies routes.

**Fix:** Add reconciliation step to Story 4.1.

### F-019: Narrative Repetition Across Stories

**Finding:** Stories 4.1-4.5 and 5.1-5.5 have significant narrative overlap in "Why This Story Exists" sections. This is not a security issue but makes review harder and increases risk of inconsistent decisions across stories.

**Fix:** Consolidate shared context into epic-level documents. Stories should reference epic context rather than duplicating.

### F-020: Logout-All User Discovery Leakage (Story 3.5)

**Story:** 3.5
**Finding:** Returns 204 No Content for valid tokens, 401 for invalid. This leaks token validity but not user existence. This is an acceptable trade-off but should be explicitly documented as a security design decision.

**Fix:** Add security design decision note to Story 3.5.

### F-021: No CSRF Protection Documentation for Token Exchange (Story 6.1)

**Story:** 6.1
**Finding:** `POST /auth/token` operates on bearer tokens (not sent by browsers automatically), so CSRF is not a concern for API consumers. However, if the endpoint is ever called from a browser context, it would be vulnerable.

**Fix:** Document the browser-context assumption in Story 6.1.

## POSITIVE FINDINGS

- **PII removal (Story 2.3):** Correct and follows OWASP recommendations
- **Entitlements reference pattern (Story 2.3):** Right approach for large permission sets
- **Token family-based reuse detection (Story 3.2):** Well-reasoned approach to the "tear" scenario
- **Shadow decision migration (Story 9.4):** Correct approach for safely migrating to JWT common-path
- **Decision matrix by endpoint type (Story 4.4):** Correctly distinguishes trust-creation from trust-evaluation routes
- **Three-layer revocation model (TTL + version + jti denylist):** Properly designed
- **Audit logging format (Story 8.3):** Comprehensive and PII-free
- **NGINX header budget analysis (Story 2.5):** Thorough and realistic

## RECOMMENDED EXECUTION ORDER (Revised)

1. Epic 1 + Epic 8 (security hardening) -- EdDSA + DPoP + typ enforcement
2. Epic 2 (claims schema) -- PII removal + entitlements hash
3. Epic 3 (token lifecycle) -- refresh rotation + binding
4. Epic 5 (versioning) -- with longer TTLs
5. Epic 6 (delegation) -- with audience validation
6. Epic 4 (hybrid authz) -- with tenant-scoped cache keys
7. Epic 7 (caching) -- with consumer hash verification
8. Epic 9 (observability) -- in parallel with all above

## FILES MODIFIED

This assessment and the subsequent patches address:
- `docs/Epics/01-asymmetric-jwks/JWT.md`
- `docs/Epics/01-asymmetric-jwks/stories/story-1.1.md`
- `docs/Epics/01-asymmetric-jwks/stories/story-1.2.md`
- `docs/Epics/01-asymmetric-jwks/stories/story-1.3.md`
- `docs/Epics/02-claims-schema-evolution/stories/story-2.3.md`
- `docs/Epics/03-token-lifecycle/stories/story-3.1.md`
- `docs/Epics/03-token-lifecycle/stories/story-3.3.md`
- `docs/Epics/03-token-lifecycle/stories/story-3.5.md`
- `docs/Epics/04-hybrid-authz-model/stories/story-4.1.md`
- `docs/Epics/04-hybrid-authz-model/stories/story-4.3.md`
- `docs/Epics/05-token-versioning/stories/story-5.2.md`
- `docs/Epics/06-delegation-act/stories/story-6.1.md`
- `docs/Epics/06-delegation-act/stories/story-6.3.md`
- `docs/Epics/08-security-hardening/stories/story-8.1.md`
- `docs/Epics/08-security-hardening/stories/story-8.2.md`
- `docs/Epics/INDEX.md`
