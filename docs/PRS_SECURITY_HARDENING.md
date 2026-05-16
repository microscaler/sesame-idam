# PRS: Security Hardening — JWT Authorization Threat Model

**Date:** 2026-05-16  
**Methodology:** Malicious actor mindset. For each component, asks "How can I break this?" rather than "Is this implemented correctly?" Focuses on what happens when a determined attacker encounters this design.

**Scope:** JWT authorization load mitigation design — Epics 1-9, 37 stories, covering hybrid authz model, token lifecycle, claims schema, delegation, token versioning, and security hardening.

**Reference Design:** `docs/Sesame-idam_authorisation_load_mitigation_with_JWT_claims.md`  
**Security Assessment:** `docs/Epics/security-assessment-JWT-authz.md` (21 findings: F-001 through F-021)

---

## Summary of Critical Holes (Prioritized for Attack)

| # | Hole | Impact | Difficulty | Target Stories |
|---|------|--------|------------|----------------|
| 1 | Entitlements hash not verified (F-007) | Tenant bleed via Redis cache poisoning | Easy | 4.3, 4.4 |
| 2 | Version check fail-open | Stale permissions after cache expiry | Easy | 5.2, 4.4 |
| 3 | No token binding (F-004) | Full session takeover from stolen token | Trivial | 8.2, all |
| 4 | No refresh token binding (F-015) | Replay stolen refresh from any device | Trivial | 3.1 |
| 5 | Tenant ID not validated in middleware | Cross-tenant data exfiltration | Easy | 4.2 |
| 6 | Login endpoint DoS on authz-core | Service degradation | Easy | 4.4 |
| 7 | Permission injection via JWT claims | Privilege escalation | Medium | 2.2, 4.2 |
| 8 | Version check only for elevated risk | Permission changes delayed | Medium | 5.2 |
| 9 | MFA step-up doesn't protect session | Partial MFA bypass | Medium | 6.3 |
| 10 | No rate limiting on auth endpoints | Brute force / DoS | Trivial | all |
| 11 | Self-service read ownership check is path-parameter dependent | User enumeration / privacy breach | Easy | 4.4 |
| 12 | Single point of failure in authz-core | Partial outage on authz-core failure | Medium | 4.x, 1.1 |
| 13 | OIDC Discovery / JWKS endpoint information leakage | Attack reconnaissance | Easy | 1.2 |
| 14 | Service restart key regeneration creates signature gap | Arbitrary token forgery during grace period | Medium | 1.1 |
| 15 | Token exchange scope intersection can grant excess privileges | Cross-org privilege escalation | Medium | 6.1 |
| 16 | extract_jti helper disables signature validation | Trust decision from unvalidated token | Easy | 1.3, 4.2, 4.3, 5.3, 8.4 |
| 17 | Token exchange audience merging is missing (F-003, F-012) | Cross-service token misuse | Easy | 6.1 |
| 18 | MFA step-up token version not bumped (F-006 incomplete) | Pre-MFA token replay attack | Medium | 6.3, 5.1 |
| 19 | Password reset endpoint not defined in any story | Brute-force account takeover | Trivial | all |
| 20 | act.chain depth not bounded | Stack exhaustion / DoS | Easy | 6.1 |
| 21 | Fallback cache single-flight not specified | Cache miss thundering herd | Medium | 4.3 |
| 22 | `SesameAuthzClaims.permissions` is a raw `Vec<String>` with no signature | Trust boundary illusion | Medium | 2.2, 4.2 |

The most immediately exploitable holes are #1, #2, #4, #10, and #16 because they require no special privileges — any authenticated user or attacker can trigger them. The most damaging holes are #3 (no token binding) for complete session takeover, and #5 (tenant ID validation) for multi-tenant data exfiltration at scale.

---

## HOLE 1: Entitlements Hash Not Verified (F-007)

**Target:** Story 4.3 (Selective Online Fallback) + Story 4.4 (Route-Specific Authz) + Story 2.3 (PII removal)

The JWT schema includes `entitlements_hash` in `SesameAuthzClaims` but consumer-side verification is explicitly missing in the security assessment (F-007). The design relies on Redis fallback cache serving entitlements snapshots, but without hash verification:

**Exploit Path (Detailed):**
1. Attacker identifies the fallback cache pattern from Story 4.3: `authz_fallback:{blake3_hash}` keys with per-route TTL
2. Attacker either (a) compromises Redis through another vulnerability, (b) is a malicious insider with Redis access, or (c) observes cache keys and predicts hash values via the deterministic key generation in Story 4.3's `generate_fallback_cache_key()`
3. Attacker writes a modified entitlements snapshot to a predictable cache key
4. A service request hits the cache, receives the modified snapshot containing additional permissions
5. Without hash verification against `sx.entitlements_hash` in the JWT, the consumer accepts the modified snapshot
6. Result: **tenant bleed via cache poisoning** — attacker gains unauthorized permissions

**Exploit Path (Indirect):** An attacker doesn't even need Redis write access. If the `entitlements_ref` points to a shared data store, an attacker with write access to that store can modify the entitlements snapshot. The `entitlements_hash` in the JWT is designed to prevent this — but Story 4.3's cache handler never compares the hash from the JWT against the hash of the fetched snapshot.

**Impact:** Multi-tenant data isolation breach. The entire `entitlements_ref`/`entitlements_hash` pattern only works if:
- (a) the hash algorithm is standardized (SHA-256? BLAKE3? Epic 2 notes this is unspecified)
- (b) the hash is verified at the consumer (NOT implemented)
- (c) the reference points to a verifiable source (unclear)

**Why this matters for Sesame specifically:** The tenancy model relies on three layers of isolation (BRRTRouter middleware, SesameExecutor, RLS policies). The fallback cache operates OUTSIDE all three layers — it's a shared Redis cache that any tenant's request can read from. If an attacker can poison this cache, they bypass ALL three isolation layers because the cache result is trusted without verification.

**Fix:** 
1. Add consumer-side hash verification step to Story 4.3's fallback cache handler: after fetching the cached entitlements snapshot from Redis, compute `blake3(snapshot_bytes)` and compare against `claims.sx.entitlements_hash`. If mismatch, reject the cache and call authz-core directly.
2. Standardize hash algorithm in Story 2.3 (recommend SHA-256 for compatibility, BLAKE3 for performance)
3. Document the hash verification as a mandatory step in Story 4.3 acceptance criteria
4. Add security regression test: "Given a modified entitlements snapshot in Redis, assert the handler rejects the cache and falls back to authz-core"

---

## HOLE 2: Version Check Is Fail-Open AND Bypassable

**Target:** Token versioning (Story 5.2) + Story 4.4

The version check reads from a Redis cache with TTL of 15-60 seconds. When the cache is empty (expired), the check is skipped (fail-open). The wiki explicitly states: "After TTL expiry, the cache is empty and the version check is skipped (fail open). Worst-case stale window: 60s (cache expired) + 5min (token TTL) = ~5.5 minutes."

**Exploit Path (Detailed):**
1. Attacker compromises a user's permissions (role change: `admin` → `user`)
2. Version is bumped in Redis: `authz_ver:{sub}` = 43 (was 42)
3. Attacker's token has `ver: 42`
4. Attacker waits for cache TTL to expire (15 seconds for subject, 60 seconds for tenant)
5. Cache is empty, version check is SKIPPED (fail-open by design in Story 5.2)
6. Attacker's stale token is accepted — they still have `admin` role
7. Attacker floods benign requests to keep cache populated, extending the window indefinitely

**Exploit Path (Even Worse — Elevated-Risk Only):** Story 4.4 gates version checks behind `sx.risk == "elevated"`. Normal JWT-only routes SKIP version checks entirely. An attacker who is classified as "normal" risk can act with stale permissions for the ENTIRE token TTL (5 minutes) on ALL jwt-only routes. Only elevated-risk delegated/admin actions check the version, and those are the minority of endpoints.

**Impact:** A user whose permissions were revoked can still act for up to their entire token TTL (5 minutes) on jwt-only routes. The version bump mechanism is PARTIALLY BROKEN — it only works for elevated-risk routes. This means:
- Admin permission changes are effective only on delegated/admin routes
- Normal operations (self-service reads/writes) continue with stale permissions
- The versioning system provides false confidence — it's not a complete revocation mechanism

**Design flaw in Story 5.2:** The wiki acknowledges the TTL strategy creates a gap but treats it as acceptable: "This is shorter than token TTL (300s), meaning after a version bump, stale tokens are rejected for the cache TTL duration. After TTL expiry, the cache is empty and the version check is skipped (fail open)." This means the system is deliberately designed to fail open for version checks. The gap is not a bug — it's a design choice that trades revocation freshness for Redis availability.

**Fix:**
1. Increase tenant version TTL to 5 minutes (matching token TTL) — but this alone doesn't fix the fail-open nature
2. Use push invalidation (Story 5.4) to keep cache populated — this is the correct fix, as it maintains the version in cache even during Redis stress
3. Apply version checks to ALL route types, not just elevated-risk ones — remove the `sx.risk == "elevated"` gate from the version check logic
4. Consider a "soft fail-open": if version cache is empty, perform a lightweight DB check instead of skipping entirely. The DB check is a single `SELECT` on `users` table for `authz_version` — much cheaper than a full authz-core call
5. Add monitoring: alert when version cache miss rate exceeds threshold (indicates possible Redis outage or version cache exhaustion)

---

## HOLE 3: No Token Binding for Browser Clients (F-004)

**Target:** Epic 8 (Security Hardening) — Story 8.2

F-004 identifies DPoP as missing. The security assessment notes: "TLS-SNI token binding only works when the client extracts SNI from the TLS handshake (server-side). DPoP works at the application layer and protects against proxy-based token theft even when TLS terminates at a load balancer."

**Exploit Path (Detailed):**
1. Attacker steals a user's access token through ANY vector:
   - XSS on a web application consuming the API
   - CORS misconfiguration allowing cross-origin token access
   - Network sniffing on an unencrypted connection (man-in-the-middle)
   - Log file exposure (the JWT document warns: "Do NOT log raw access tokens")
   - Memory dump from the client device
2. Attacker uses the stolen token from any device, any IP, any network
3. The token is a bearer token — possession = authorization (per RFC 9068)
4. No sender-constraining means no way to detect "wrong device"
5. Attacker has full access with the stolen token until it expires (5 minutes) or is detected through other means (version bump, denylist, etc.)

**Why this is the most damaging hole:** Every other design element in the JWT system is REACTIVE — it detects compromise after the fact. Version checking catches permission changes. Denylist catches urgent revocations. Refresh token rotation detects replay. But none of these PREVENT token misuse from a stolen token. Without token binding, the system has no PREVENTIVE mechanism for browser clients.

**Impact:** Complete session takeover from stolen token. The attacker doesn't need to compromise any server — they just need the token. And tokens are designed to be long-lived enough that 5 minutes is plenty of time for an attacker to:
- Exfiltrate sensitive data
- Perform destructive actions
- Establish persistence (create API keys, change email, etc.)

**Why DPoP specifically:** RFC 9449 DPoP binds tokens to a client-specific cryptographic key pair. The client must sign each request with their DPoP proof key, and the server validates that the proof key matches the key referenced in the token. If an attacker steals the token but not the DPoP private key, the stolen token is useless.

**Fix:**
1. Replace Story 8.2 with DPoP implementation — TLS-SNI is insufficient for browser clients
2. DPoP proof key is client-side (generated by the browser or native app) — the server never sees the private key
3. Required for: all API endpoints, not just high-risk ones
4. Document in Story 8.5: "Client must present a DPoP proof with each request. Server validates DPoP signature matches the access token's `htm` (HTTP method) and `htu` (HTTP URI)"
5. Consider a phased rollout: DPoP for high-risk routes first (Story 8.5 says "Required for: API key validation, token exchange, impersonation, admin actions"), then expand to all routes

---

## HOLE 4: No Refresh Token Binding (F-015)

**Target:** Story 3.1 (Refresh Token Rotation)

Token binding (Epic 8, Story 8.5) applies to access tokens but refresh tokens are NOT bound. The security assessment (F-015) explicitly calls this out: "Token binding (Epic 8) applies to access tokens but refresh tokens are not bound. A stolen refresh token can be replayed from any client. Refresh token binding (DPoP or sender-constrained) is essential for family-based revocation to work properly."

**Exploit Path (Detailed — The "Tear" Scenario Amplified):**
1. Attacker intercepts a refresh token (from network sniffing, log access, memory dump, or XSS)
2. Attacker replays the refresh token from ANY device/network — no binding means the server cannot distinguish the attacker from the legitimate user
3. Each replay generates a new access token (rotation — the refresh token is "used up" and a new one is issued)
4. The legitimate user's refresh token becomes "the old one" in the denylist (Story 3.1)
5. Only THEN is the family revoked — this is the reuse detection that Story 3.2 implements
6. BETWEEN step 3 (first replay) and step 5 (reuse detection), the attacker has FULL ACCESS for as long as the refresh token is valid (7-30 days)

**Why this matters:** The "tear" scenario in Story 3.2 is the primary defense against stolen refresh tokens. But the tear detection is REACTIVE — it only detects compromise after the attacker has already used the stolen token. Without refresh token binding, the tear detection window is 7-30 days (the entire refresh token lifetime). With binding, the window is reduced to the time between theft and reuse detection, which could be minutes if the legitimate user also refreshes.

**Compounding factors:**
- The refresh token family model (Story 3.2) groups tokens by session, but without binding, the attacker can use the same family token from a different device
- The denylist TTL in Story 3.1 is 24 hours for normal rotation — meaning even after reuse detection, the attacker's tokens remain in the denylist for 24 hours
- The cross-session notification (F-005) is mentioned but not specified — how does the user know their account is compromised?

**Impact:** Stolen refresh tokens work from any device until reuse is detected. The "tear" scenario detection is the ONLY defense, and it is reactive. Without binding, the system cannot prevent replay from a different device.

**Fix:**
1. Add refresh token binding (DPoP or sender-constrained) to Story 3.1
2. Store the DPoP proof key (or sender identity) in the refresh token metadata in Redis
3. On refresh, validate that the presented proof key matches the stored key
4. On mismatch, revoke the entire family and require re-authentication
5. Document in Story 8.2: "Refresh tokens are DPoP-bound. A stolen refresh token without the matching DPoP proof key cannot be replayed"

---

## HOLE 5: Tenant ID Validation Is Not Guaranteed

**Target:** Tenancy model + Story 4.2 + Story 2.4

The wiki says: "Tenant validation is critical: `claims.tenant_id == X-Tenant-ID` — mismatch = 401." But this is listed as a *requirement*, not a *guaranteed implementation*. The tenancy model wiki confirms the three-layer isolation (BRRTRouter middleware, SesameExecutor, RLS), but the JWT claims schema migration creates a transition gap.

**Exploit Path (Detailed — The Migration Window):**
1. The system is migrating from the old JWT schema (no `tenant` claim) to the new schema (Story 2.4 adds `tenant` to JWT claims)
2. Existing valid tokens from the old schema LACK the `tenant` claim
3. If middleware cannot validate tenant from JWT for old tokens, it must fall back to X-Tenant-ID header
4. X-Tenant-ID header is CLIENT-CONTROLLED — an attacker can send any value
5. Attacker crafts a request with a valid old-schema JWT (Tenant A) and `X-Tenant-ID: Tenant B` header
6. Middleware uses the attacker-chosen tenant context (Tenant B)
7. Queries run against Tenant B's data
8. RLS bridge may provide a safety net, but it is "partially-verified" and "actual RLS helper SQL is not yet in the repo" (from `topic-rls-bridge.md` gaps)

**Exploit Path (Even Without Migration — If Middleware Doesn't Enforce It):**
1. Attacker has any valid JWT (from any source)
2. Attacker sends requests with `X-Tenant-ID: any-tenant` header
3. If the middleware doesn't strictly validate `claims.tenant_id == X-Tenant-ID`, queries run against the attacker-chosen tenant
4. Result: **cross-tenant data exfiltration**

**Impact:** Cross-tenant data exfiltration. The RLS bridge is a failsafe, not a primary control. If application-layer tenant validation fails, the database layer must catch it — but the RLS implementation is incomplete ("actual RLS helper SQL is not yet in the repo"). This means the system has NO reliable cross-tenant isolation during the migration window.

**Why this matters for Sesame:** The entire business model relies on tenant isolation. Each customer (hauliage, rerp) expects their data to be completely isolated from other tenants. A breach of this isolation is a business-critical security failure — not just a technical vulnerability.

**Fix:**
1. Enforce tenant validation as the FIRST step in Story 4.2 middleware — BEFORE any handler logic
2. For tokens without `tenant` claim (during migration):
   - Look up the JWT's `sub` in the database to resolve its tenant
   - Compare the resolved tenant against `X-Tenant-ID` header
   - If mismatch, reject with 401
3. For tokens WITH `tenant` claim (after migration):
   - Direct comparison: `claims.tenant_id == X-Tenant-ID`
4. Ensure RLS helper SQL is implemented and tested before production (blocker for production deployment)
5. Add security regression test: "Given a JWT for Tenant A with X-Tenant-ID: Tenant B, assert the handler returns 401 TenantMismatch"
6. Document the migration strategy: how long will old tokens be valid? (TTL-based: 5 minutes for access tokens, 7-30 days for refresh tokens)

---

## HOLE 6: Login Endpoint DoS on authz-core

**Target:** Login flow + Story 4.4

Login routes "CREATE trust, not evaluate it." They are not protected by JWT middleware. But the login handler calls `authz-core /principal/effective` for JWT claim enrichment (Story 4.4, Login, Callback, OTP section).

**Exploit Path (Detailed):**
1. Attacker floods `/auth/login` with valid credentials (or valid password guesses from a credential stuffing attack)
2. Each login triggers an authz-core call: `POST /api/v1/am/principal/effective {user_id, org_id}`
3. authz-core resolves roles + permissions from PostgreSQL (EXPENSIVE — involves multiple queries, role inheritance chain walking, permission aggregation)
4. Since login routes are excluded from JWT middleware (Story 4.4 explicitly states "Login routes are NOT protected by JWT common-path authz"), there is no JWT-based rate limiting
5. Result: **authz-core DoS via login flooding** — the authz-core service becomes overwhelmed

**Impact:** Service degradation. Authz-core becomes the bottleneck for all new sessions. Even if individual login requests succeed, the authz-core service may become overwhelmed, causing:
- Slow login response times for legitimate users
- Timeout errors for authz-core calls
- Cascading failure to identity-login-service (which depends on authz-core at login time)

**The design gap:** The JWT document acknowledges "bursty during sign-in and recovery flows" but provides no mitigation. The login flow wiki states "Password hashing is the bottleneck. CPU-bound operation. Needs to scale vertically." — but the authz-core call after password hashing is equally expensive and has no documented scaling strategy.

**Fix:**
1. Add rate limiting to the login endpoint itself (e.g., 10 requests/minute per IP) at the gateway level (NGINX/API Gateway)
2. Consider a "login throttle" in identity-login-service: per-IP, per-email, and per-IP:email combinations
3. Document the rate limit policy in Story 4.4's login route section
4. Consider caching the `/principal/effective` result in Redis for recently-authenticated users (1-5 minute TTL) to reduce load during login floods
5. Add monitoring: alert on login endpoint QPS exceeding threshold (indicates potential DoS)

---

## HOLE 7: Permission Injection via JWT Claims

**Target:** Story 2.2 (TokenClaims structs) + Story 4.2 (JWT middleware)

The JWT middleware validates the signature, then trusts `sx.permissions` as an authoritative list. The permissions are a `Vec<String>` in `SesameAuthzClaims` (from Story 2.2's target Rust types). The handler code in Story 4.4's examples checks `claims.sx.permissions.contains(&"email:write".to_string())`.

**Exploit Path (Detailed — Key Compromise):**
1. The private signing key is compromised (memory dump, process leak, insider threat, or server breach at identity-login-service)
2. Attacker crafts a JWT with arbitrary permissions (e.g., `sx.permissions = ["admin:all", "users:manage", "api_keys:all"]`)
3. The JWT is valid (correct signature, correct `kid`, not expired)
4. Middleware validates the signature — checks pass
5. Handler checks `claims.sx.permissions.contains(&"email:write".to_string())` — passes
6. Handler proceeds to execute the sensitive action
7. Result: **privilege escalation via forged JWT**

**Exploit Path (Even Without Key Compromise — If Signing Code Has a Bug):**
1. There is a bug in the JWT signing code (e.g., incorrect claim serialization, integer overflow in version increment, incorrect audience construction)
2. Attacker crafts a JWT that exploits this bug (e.g., sets `ver = 0` to bypass version check, or sets `aud` to match any service)
3. The JWT passes validation (the bug causes incorrect behavior)
4. Result: **privilege escalation via JWT construction bug**

**The deeper hole:** The design assumes asymmetric signing prevents injection. This is TRUE for the signing operation — no one can forge a signature. But the design also assumes that the signing code is correct, and that the signing key is not compromised. These are operational assumptions, not cryptographic guarantees.

**Why `sx.permissions` is a `Vec<String>`:** Story 2.2's `SesameAuthzClaims` struct has `pub permissions: Vec<String>` — a plain array of permission strings. This means:
- No tamper-proofing beyond the JWT signature
- No hash verification (which is the whole point of `entitlements_hash` — Story 2.3)
- No server-side re-verification at the handler level

**Impact:** Privilege escalation. If the signing key is compromised, the attacker can generate tokens with ANY permissions. The JWT becomes the single source of truth for authorization, with no backup verification. This is dangerous because:
- The JWT is a "common path optimization" (as the design doc states)
- But handlers treat it as THE authoritative source
- There is no "verify JWT claims against canonical source" step for high-consequence actions

**Fix:**
1. For high-consequence routes (email:write, admin:create_org, api_key:create/revoke, role:assign), always verify permissions against the canonical source (database or authz-core), even if JWT claims suggest permission exists
2. The JWT claims are a "common path optimization" for the 95% of requests that don't need verification — but for the 5% that do, there MUST be a fallback
3. Document this as a design rule: "JWT claims are an optimization, not the authoritative source"
4. Add security regression test: "Given a JWT with fabricated permissions, assert high-consequence routes still require online verification"
5. Consider adding a `permissions_hash` (separate from `entitlements_hash`) that covers the `sx.permissions` array — this allows handlers to verify the permission list hasn't been tampered with

---

## HOLE 8: Version Check Only for Elevated Risk

**Target:** Story 5.2 + Story 4.4

The version check is gated behind `sx.risk == "elevated"` (from Story 4.4's delegated/admin actions section: "Version check (if elevated risk)"). Normal routes (jwt-only, jwt-with-fallback for low-risk) SKIP version checks entirely.

**Exploit Path (Detailed):**
1. Attacker is a regular user (risk = "normal" or risk is absent)
2. Attacker's permissions are revoked (admin changes role from `admin` to `user`)
3. Version is bumped in Redis: `authz_ver:{sub}` = 43
4. Attacker's token has `ver: 42`
5. Attacker makes requests to jwt-only routes (e.g., `GET /api/v1/identity/users/me`, `GET /api/v1/identity/preferences`)
6. Version check is SKIPPED because `sx.risk != "elevated"`
7. Attacker's stale token is accepted for the ENTIRE token TTL (5 minutes)
8. During those 5 minutes, the attacker has admin-level access on jwt-only routes

**Even more concerning:** jwt-with-fallback routes for low-risk writes (e.g., `PUT /api/v1/identity/preferences`) also SKIP version checks. Only delegated/admin routes with `sx.risk == "elevated"` check the version.

**Impact:** Permission changes are effective only on elevated-risk routes. Normal operations continue with stale permissions. The version bump mechanism is PARTIALLY BROKEN — it only works for the subset of endpoints classified as elevated-risk.

**Why this gate exists (and why it's wrong):** The design rationale appears to be that version checks add latency (Redis lookup) and should only be done for high-risk routes. But this creates a security gap where normal-risk users can act with stale permissions. The correct approach is to version-check ALL routes, and optimize the check (cache with short TTL) rather than skip it.

**Fix:**
1. Apply version checks to ALL route types — remove the `sx.risk == "elevated"` gate
2. Keep the cache optimization (version cache with 15-60s TTL) — this is the correct trade-off
3. Add a "soft fail-open" (see Hole 2 fix) — if version cache is empty, perform a lightweight DB check
4. Document in Story 5.2: "Version checks apply to ALL route types, not just elevated-risk routes. The risk classification gates MFA requirements (Story 6.3), not version validation"

---

## HOLE 9: MFA Step-Up Does Not Protect Session

**Target:** Story 6.3 (Step-Up MFA) + Story 3.1 (Refresh Token Rotation)

When a user completes step-up MFA, they get a new access token with `sx.mfa_verified = true`. The old refresh token is denylisted (F-006 Fix in Story 6.3). But this only protects against the old refresh token — not against already-issued access tokens.

**Exploit Path (Detailed — The "Pre-MFA Token" Attack):**
1. User logs in (no MFA, or step-up not completed yet). They have a valid access token with `sx.mfa_verified = false`
2. Attacker has the user's refresh token (stolen before step-up)
3. Attacker uses the stolen refresh token to obtain a new access token (before the user does step-up MFA)
4. The attacker's access token has `sx.mfa_verified = false` but ALL the user's permissions
5. User completes step-up MFA
6. New access token is issued with `sx.mfa_verified = true`
7. Old refresh token is denylisted (F-006)
8. BUT: the attacker already has a valid access token from step 3, and this token is NOT revoked
9. The attacker can use this token to perform actions — including non-MFA-protected ones

**The deeper hole (from Security Assessment F-006):** The F-006 fix adds the old refresh token to the denylist. This prevents the attacker from using the REFRESH TOKEN after step-up. But if the attacker ALREADY used the refresh token to get an access token BEFORE step-up, the attacker has a valid access token that is NOT revoked.

**Even deeper hole:** The step-up MFA only affects the MFA-verified claim, not the permissions themselves. Even with `mfa_verified: false`, the attacker still has access to non-MFA-protected routes. The MFA step-up only protects specific high-consequence actions (admin:create_org, org:config:update, admin:impersonate, api_key:create, api_key:revoke, role:assign). It does NOT protect the entire session.

**Impact:** Partial MFA bypass. Attacker retains access to all non-MFA-protected routes. The MFA step-up is a partial defense, not a full session reset.

**Fix:**
1. On step-up MFA completion, invalidate ALL existing access tokens from the user's session — not just the refresh token
2. Bump the token version (`ver`) on step-up MFA completion (Story 5.1 says version is bumped on "privilege change" — step-up MFA is effectively a privilege change for the session)
3. Issue a fresh token with new `ver` and `sid`
4. This ensures the attacker cannot continue using pre-MFA tokens (they will be rejected by version check on the next request)
5. Add to Story 6.3 acceptance criteria: "On step-up MFA completion, the token version is bumped, invalidating all existing access tokens from the session"

---

## HOLE 10: No Rate Limiting on Auth Endpoints

**Target:** All stories

The JWT document discusses performance extensively but rate limiting is barely mentioned. The security assessment (F-009) identifies JWKS rate limiting as missing. The gaps are wider:

**Detailed Breakdown:**
1. **Login endpoint (`/auth/login`):** No rate limit. Attacker can brute-force passwords or perform credential stuffing. The password hashing step (bcrypt/scrypt) is CPU-bound, making login flooding a cost-effective DoS against the identity-login-service.

2. **Password reset:** NOT MENTIONED in any story. The login flow wiki lists "Email OTP," "Phone OTP," "Social OAuth," "Email Magic Link," "SMS Magic Link" — but no dedicated password reset endpoint. An attacker can flood any OTP/magic link endpoint to send unlimited codes.

3. **Email OTP / Phone OTP:** Sending unlimited OTPs costs money (SMS) and enables phishing (email). No rate limit is specified.

4. **Token refresh (`/auth/refresh`):** No rate limit. Attacker can hammer refresh with stolen tokens. Each refresh triggers a Redis lookup and potentially an authz-core call (if version check is enabled).

5. **Step-up MFA:** 10 req/min is mentioned in Story 6.3's security considerations, but the acceptance criteria don't include rate limit enforcement. The unit tests mention "MFA code rate limiting: Given 10 failed MFA attempts in 1 minute, assert the 11th attempt is rate-limited (returns 429 Too Many Requests)" — but this is a test, not a guaranteed implementation.

6. **API key validation:** Called on every request in the hybrid model. No rate limit means it's a DoS vector. An attacker can flood API key validation endpoints to overwhelm the api-keys service.

7. **Token exchange (`/auth/token`):** No rate limit. Attacker can use token exchange to bypass rate limits on the login endpoint (F-021 security regression test mentions "Actor cannot use token exchange to bypass rate limits" but no implementation is specified).

8. **JWKS endpoint (`/.well-known/jwks.json`):** No rate limit (F-009). 6 services fetching every 5 minutes plus external OAuth consumers could receive hundreds of requests/second from an attacker.

**Impact:** Brute force attacks, SMS phishing, service degradation. Every auth endpoint is a potential entry point for abuse. Without rate limiting, the system has no defense against automated attacks.

**Fix:**
1. Add rate limiting to ALL auth endpoints at the gateway level (NGINX/API Gateway) before requests reach the application
2. Document rate limit policies in OpenAPI specs for each endpoint
3. Implement per-endpoint rate limits:
   - Login: 10 req/min per IP
   - Refresh: 30 req/min per user
   - Step-up MFA: 10 req/min per user (already in tests, enforce in implementation)
   - Token exchange: 10 req/min per IP (same as login)
   - JWKS: 100 req/s (F-009)
   - OTP endpoints: 5 req/min per email/phone
4. Consider using a sliding window algorithm (not fixed window) to prevent rate limit evasion by timing requests at window boundaries
5. Add monitoring: alert on rate limit violations (indicates potential attack)

---

## HOLE 11: Self-Service Read Ownership Check Is Path-Parameter Dependent

**Target:** Story 4.4, Self-Service Reads

The ownership check is: `claims.sub == request.user_id`. For `GET /api/v1/identity/users/me`, the user_id is implicit (it's the JWT subject). But for `GET /api/v1/identity/users/{user_id}`, the user_id comes from the URL path.

**Exploit Path (Detailed):**
1. Attacker has a valid JWT with `sub: "alice"` and permissions to view profiles
2. Attacker requests `GET /api/v1/identity/users/bob` (changing the path parameter)
3. The ownership check `claims.sub == request.user_id` fails for this path
4. BUT: `/api/v1/identity/users/{id}` is classified as `jwt-with-fallback` (Story 4.4, Identity Resolution section)
5. The fallback calls authz-core for authorization
6. If authz-core allows user A to view user B's profile (e.g., same org), the request succeeds
7. An attacker can enumerate all users in the org by iterating user IDs
8. Result: **user enumeration and privacy breach**

**Why this is particularly dangerous:** The `jwt-with-fallback` classification means the endpoint is designed to work for same-org visibility. An attacker with a valid JWT can enumerate all users in their org by requesting `GET /api/v1/identity/users/{id}` for every possible UUID. This is not a vulnerability in the authorization logic — it's a consequence of the endpoint being designed for org-scoped visibility.

**Impact:** User enumeration and privacy breach. The ownership check is only effective for `/me` endpoints. Arbitrary user lookups rely on authz-core, which may allow same-org visibility. This means:
- An attacker can discover all users in their org
- An attacker can discover user profiles (email, name, phone) for all org members
- An attacker can correlate user IDs across systems (if user IDs are stable identifiers)

**Fix:**
1. Add explicit documentation that `/api/v1/identity/users/{user_id}` requires the same org membership or admin permission
2. Add rate limiting to user lookup endpoints to prevent enumeration (e.g., 100 req/min per user)
3. Consider removing public user lookup endpoints entirely — replace with a search endpoint that returns only a limited subset of fields (e.g., just names, no email/phone)
4. Add to Story 4.4 acceptance criteria: "User lookup endpoint returns 403 Forbidden if the requested user is not in the same org and the caller is not an admin"
5. Add security regression test: "Given a JWT for user alice in org A, assert requests to get users from org B return 403"

---

## HOLE 12: Single Point of Failure in authz-core

**Target:** Epic 4 design

The entire hybrid model depends on authz-core being available for fallback routes and for login-time JWT enrichment. If authz-core goes down, the impact is asymmetric:

**Detailed Impact Analysis:**
1. **jwt-only routes continue working** (good — they only need JWT validation)
2. **jwt-with-fallback routes fail** (503 — they need authz-core for the fallback path)
3. **online-only routes fail** (503 — they always need authz-core)
4. **New logins fail** (authz-core `/principal/effective` is unavailable — identity-login-service cannot enrich JWT claims)
5. **New tokens cannot be issued** (no login means no new tokens)
6. **Refresh tokens still work** (for existing tokens — JWT validation is local)

**Result:** The system enters a "graceful degradation" mode where only read-only, self-service operations work for EXISTING sessions. All writes, all new sessions, all admin actions are blocked. This is NOT a full outage, but it IS a partial outage that affects the most critical operations.

**The design gap:** The design doesn't address what happens during authz-core downtime for the fallback cache. If authz-core is down, the fallback cache can still serve cached results. But the cache TTL is 5-30 seconds, so after a few minutes the cache expires and requests start failing. The design doesn't specify a "cache-only" mode for degraded operation.

**Impact:** Partial outage. The system is not resilient to authz-core failure beyond a few minutes. During the outage:
- Users can read their own data (jwt-only routes)
- Users CANNOT update their data (jwt-with-fallback routes fail)
- Admins CANNOT perform admin actions (online-only routes fail)
- New users CANNOT log in (login flow fails)
- API consumers using M2M keys still work (api-keys service is independent — Story 5 says "Only cross-service dependency: login → authz-core")

**Fix:**
1. Implement a "cache-only" fallback mode where the service serves from the Redis cache even when authz-core is unavailable
2. Document the degradation behavior: which routes work, which fail, and why
3. Add monitoring for authz-core health (HTTP health check endpoint, latency metrics, error rate metrics)
4. Alert on authz-core downtime: "Alert if authz-core returns > 10% errors or latency > 5s for 1 minute"
5. Consider circuit breaker pattern: if authz-core is failing, stop calling it for fallback routes and serve from cache only
6. Document the circuit breaker behavior: "When circuit breaker is open, fallback routes serve from cache only. New cache entries are not written during outage"

---

## HOLE 13: OIDC Discovery / JWKS Endpoint Information Leakage

**Target:** Story 1.2 (JWKS endpoint)

The JWKS endpoint is public (correct per RFC 7517) but the document doesn't discuss the information leakage implications:

**Detailed Analysis:**
1. **Metadata endpoint abuse:** The `/.well-known/openid-configuration` endpoint leaks issuer, jwks_uri, and supported algorithms. An attacker learns:
   - The issuer URL (useful for token forgery attempts)
   - The JWKS URI (useful for key enumeration)
   - The supported algorithms (EdDSA + ES256, per Story 1.1)
   - Token lifetimes (5 minutes for normal, 1-3 minutes for admin, per Story 3.3)
   - The introspection endpoint (if Story 4.5 is implemented)

2. **JWKS enumeration:** An attacker could scan JWKS for `kid` patterns and correlate with known key rotation schedules. Story 1.1 uses `key-{year}-{month}-{index}` for `kid` generation. An attacker who observes this pattern can predict future key IDs and prepare tokens signed with predicted keys (if the private key was compromised).

3. **Algorithm confusion:** If both EdDSA and ES256 are served (Story 1.1), an attacker could try algorithm confusion attacks if the consumer accepts both without strict validation. RFC 8725 requires consumers to validate algorithms from an allow-list and reject unexpected algorithms.

**Impact:** Information disclosure. Attackers learn the key rotation schedule, algorithm configuration, and issuer identity — useful for planning targeted attacks.

**Fix:**
1. Add rate limiting (100 req/s) to Story 1.2 (as recommended in F-009)
2. Document the rate limit policy in OpenAPI
3. Consider restricting `openid-configuration` to known consumers only (via IP allow-list or API key)
4. Document the algorithm negotiation policy in Story 1.1: "Consumers MUST accept both EdDSA and ES256 from the JWKS allow-list. The `alg` claim in the signed JWT header will always be EdDSA for new tokens."
5. Consider adding `kid` obfuscation: instead of `key-{year}-{month}-{index}`, use a random `kid` that maps to a key index internally. This prevents key rotation schedule prediction.

---

## HOLE 14: Service Restart Key Regeneration Creates Signature Gap

**Target:** Story 1.1

Service restart generates a NEW key pair. The old private key is dropped from memory. But tokens signed with the old key remain valid until their `exp`.

**Exploit Path (Detailed — The "Old Key" Attack):**
1. Old private key is compromised (memory dump, process leak, or insider threat)
2. Attacker crafts tokens with arbitrary permissions signed with the old key
3. Service restarts, generating a new key pair
4. The old key is removed from JWKS after the grace period (1 hour)
5. During the grace period, the crafted tokens are accepted (old key is still in JWKS)
6. After the grace period, the tokens are rejected — but the attacker may have already accessed sensitive data

**The deeper hole:** The "private key never persists" design means restarts generate new keys. This is intentional — if a private key leaks, it is rotated immediately. But the grace period allows old-signed tokens (including attacker-crafted ones) to be accepted. The grace period is 1 hour (Story 1.1), which means:
- An attacker has a 1-hour window to use forged tokens after the key is compromised
- If the service restarts before the grace period ends, the old key may never be removed from JWKS (it's still needed for existing tokens)
- If the service restarts AFTER the grace period, the old key is dropped from memory AND removed from JWKS — but the grace period already ended, so the window is missed

**Impact:** Compromised key + restart = time window for arbitrary token forgery. The "private key never persists" design is good for leak mitigation but creates a window where forged tokens are accepted.

**Fix:**
1. Add key compromise detection and immediate revocation
2. If a key is suspected compromised, remove it from JWKS immediately (before the grace period ends)
3. Monitor for unexpected `kid` usage patterns (e.g., a `kid` that hasn't been used for signing but appears in a token's header)
4. Document the compromise procedure in Story 1.1: "If a signing key is compromised, remove it from JWKS immediately and generate a new key pair"
5. Add monitoring: alert on "unexpected `kid` in token header" (indicates potential compromise)
6. Consider a "key blacklist" stored in Redis that lists compromised `kid` values — this allows immediate revocation even before the old key is removed from JWKS

---

## HOLE 15: Token Exchange Scope Intersection Can Grant Excess Privileges

**Target:** Story 6.1 (RFC 8693 Token Exchange)

The scope intersection formula is: `new_scope = subject_scope INTERSECT requested_scope INTERSECT actor_scope`. The design says "actor and subject must be in the same tenant" but doesn't address cross-org boundaries within a tenant.

**Exploit Path (Detailed):**
1. Attacker has a legitimate API key with `admin:read` scope (from any org within a tenant)
2. Attacker initiates token exchange: `subject_token = API key`, `actor_token = attacker's own token`
3. The tenant match passes (both are in the same tenant)
4. BUT: the actor is in org A and the subject is in org B (within the same tenant)
5. The scope intersection computes correctly, but the resulting token has `act` claim pointing to the attacker
6. The new token's `sub` is the subject (org B user), but the `act.sub` is the attacker (org A)
7. Result: the attacker can act as org B's user, but with their own (org A) identity as the actor

**The deeper hole:** The `can_delegate()` function in Story 6.1 checks:
- `platform_admin`: can delegate any user in their tenant
- `org_admin`: can delegate users in same org only
- `service_account` with `delegate:*` permission: delegated

If the attacker is a `platform_admin` in org A, they can delegate any user in the tenant (including org B). This means a platform admin in one org can impersonate users in other orgs within the same tenant. This may or may not be the intended behavior.

**Impact:** Cross-org privilege escalation. The token exchange could grant the attacker access to resources in a different org than the one they belong to.

**Fix:**
1. Add org-scoped validation to the token exchange: the actor's org must be validated against the subject's org
2. For `org_admin` actors, restrict delegation to users in the same org (already in Story 6.1)
3. For `platform_admin` actors, add an optional config flag: `PLATFORM_ADMIN_CROSS_ORG_DELEGATION=false` (default: true, but should be configurable)
4. Document the cross-org behavior explicitly in Story 6.1
5. Add security regression test: "Given a platform admin in org A, assert they cannot delegate a user in org B if cross-org delegation is disabled"

---

## HOLE 16: extract_jti Helper Disables Signature Validation

**Target:** Story 1.3 + Story 4.2 + Story 4.3 + Story 5.3 + Story 8.4

The current repo's `extract_jti` helper disables signature validation to extract `jti` before full validation. The JWT document explicitly warns: "must never become a trust decision path by itself." The security assessment (F-002) calls this out: "The current repo's `extract_jti` helper disables signature validation to extract `jti` before full validation. None of the validation pipeline stories address removing this pattern or enforcing validation order."

**Exploit Path (Detailed):**
1. Attacker crafts a token with a VALID `jti` value (observed from a legitimate token)
2. Attacker signs the token with a different key (or no signature at all)
3. The `extract_jti` helper extracts the `jti` WITHOUT validating the signature
4. The extracted `jti` is used to check the denylist
5. If the `jti` is in the denylist, the request is rejected (denied)
6. If the `jti` is NOT in the denylist, the request continues to full validation
7. BUT: if there's a bug in the full validation logic, or if the signature validation is bypassed, the attacker gains access with a forged token

**The deeper hole:** The `extract_jti` helper is used as a pre-validation optimization for denylist lookup. This means:
- The denylist check happens BEFORE signature validation
- If the denylist is the ONLY trust decision path (i.e., the `jti` determines allow/deny), then an attacker can forge tokens with any `jti` and the denylist lookup will return the correct result for that `jti`
- This is a TRUST DECISION PATH from an unvalidated token

**Impact:** Trust decision from unvalidated token. If `extract_jti` is used in production, an attacker could supply a valid `jti` from a forged token and potentially bypass signature validation.

**Fix:**
1. Remove or deprecate `extract_jti` in Story 8.4
2. If pre-validation jti lookup is needed for denylist, validate signature separately first
3. Add explicit comment: "WARNING: this is not a trust path — signature validation must always precede any trust decision"
4. Add to Story 1.3 and Story 4.2 acceptance criteria: "The validation pipeline always validates signature BEFORE checking the denylist"
5. Add security regression test: "Given a token with a valid jti but invalid signature, assert the request is rejected at signature validation, not at denylist lookup"

---

## HOLE 17: Token Exchange Audience Merging Is Missing (F-003, F-012)

**Target:** Story 6.1 (RFC 8693 Token Exchange)

The security assessment (F-003) notes: "RFC 8693 requires `iss`, `aud`, `iat`, `exp`, `sub`, `jti` in exchanged tokens. Story 6.1's validation pipeline does not explicitly require `aud` or `iss` in the exchanged token." F-012 notes: "The token exchange creates a new access token with merged scopes but does not validate or set the `aud` claim."

**Exploit Path (Detailed):**
1. Attacker initiates token exchange with a subject token that has `aud: ["myapp.com"]`
2. The actor token has `aud: ["support-portal.com"]`
3. The new token is issued WITHOUT the `aud` claim (because Story 6.1 doesn't set it)
4. The new token is accepted by `myapp.com` (because there's no audience check — or by any service that doesn't check audience)
5. Result: **cross-service token misuse** — a token issued for service A is accepted by service B

**Impact:** Cross-service token misuse. A token issued for service A could be accepted by service B, bypassing audience-based isolation. This is particularly dangerous in a multi-service architecture where each service has a different audience.

**Fix:**
1. Add `aud` and `iss` to TokenExchangeResponse schema and validation pipeline (per F-003)
2. The new token's `aud` should include the audience of the original token AND the audience of the actor token (per F-012)
3. Add to Story 6.1 acceptance criteria: "TokenExchangeResponse includes `iss`, `aud`, and `iat` claims per RFC 8693"
4. Add security regression test: "Given subject token audience = ['myapp.com'] and actor token audience = ['support-portal.com'], assert the response `aud` contains both: ['myapp.com', 'support-portal.com']"

---

## HOLE 18: MFA Step-Up Token Version Not Bumped (F-006 Incomplete)

**Target:** Story 6.3 (Step-Up MFA) + Story 5.1 (Token Versioning)

Story 6.3 implements F-006: on step-up MFA completion, the old refresh token is denylisted. But the token version (`ver`) is NOT bumped. This means existing access tokens from the same session remain valid.

**Exploit Path (Detailed):**
1. User logs in (no MFA verified). They have an access token with `ver: 42`
2. Attacker has the user's access token (stolen from memory, network, etc.)
3. User completes step-up MFA
4. A new access token is issued with `sx.mfa_verified = true` and `ver: 42` (NOT bumped)
5. The old refresh token is denylisted (F-006)
6. BUT: the attacker's stolen access token has `ver: 42`, which matches the current version
7. The attacker's token is NOT revoked — it has the correct version
8. The attacker still has access to the user's account until the token expires

**The hole:** Step-up MFA revokes the REFRESH TOKEN (F-006) but does NOT revoke the ACCESS TOKEN. The access token is still valid because its version matches the current version. The only way to revoke the access token is:
- Wait for it to expire (5 minutes TTL)
- Add its `jti` to the denylist (which is not done)
- Bump the version (which is not done)

**Impact:** Step-up MFA is defeated by pre-existing stolen access tokens. The F-006 fix only protects the refresh token, not the access token.

**Fix:**
1. On step-up MFA completion, bump the token version (`ver` claim) — this invalidates all existing access tokens from the session
2. Story 5.1 says "Bumped whenever user's permissions change" — step-up MFA is effectively a privilege change for the session (the user's session is now "verified")
3. The new access token should have `ver: 43` (bumped from 42)
4. The attacker's token with `ver: 42` will be rejected on the next request (version mismatch)
5. Add to Story 6.3 acceptance criteria: "On step-up MFA completion, the token version is bumped"
6. Add to Story 5.1: "Token version is bumped on: privilege change, user disabled, org deleted, step-up MFA completion, token exchange"

---

## HOLE 19: Password Reset Endpoint Not Defined

**Target:** All stories

Password reset is NOT MENTIONED in any story or design document. The login flow wiki lists variants (email+password, email OTP, phone OTP, social OAuth, email magic link, SMS magic link, dual OTP) but has no password reset endpoint.

**Exploit Path (Detailed):**
1. An attacker knows a user's email address
2. The attacker requests a password reset (via whatever mechanism exists — email OTP, magic link, etc.)
3. If no rate limiting is in place, the attacker can request unlimited password resets
4. Each reset attempt sends an email/SMS to the user, costing money and enabling phishing
5. The user receives unexpected password reset emails, which could trigger social engineering (the attacker could impersonate the support team and ask the user to "confirm" the reset)

**Why this matters:** Password reset is the most common attack vector for account takeover. Without a defined password reset flow, the system has no documented security controls for this critical operation.

**Impact:** Brute-force account takeover via password reset flooding. Without rate limiting and user notification, the system is vulnerable to:
- SMS phishing (smishing) via unlimited SMS codes
- Email phishing via unlimited magic links
- Social engineering via fake "confirm your password reset" attacks

**Fix:**
1. Define a password reset flow in the design documents (new story or section in Story 6.3)
2. Add rate limiting to the password reset endpoint (e.g., 3 requests/hour per email)
3. Send a notification to the user when a password reset is requested (email notification)
4. Require email verification for password reset (not just OTP — the user must click a link sent to their registered email)
5. Document the password reset flow in Story 4.4's route classification

---

## HOLE 20: act.chain Depth Not Bounded

**Target:** Story 6.1 (RFC 8693 Token Exchange)

The token exchange supports nested delegation with `act.chain` (Story 6.1: "Nested delegation includes `act.chain` for audit"). But there is no mention of a maximum chain depth.

**Exploit Path (Detailed):**
1. Attacker initiates a token exchange chain: tool_1 → admin_1 → user_1 → admin_2 → user_2 → ... (deeply nested)
2. Each level adds to the `act.chain`
3. If the chain is 100+ levels deep, the `act.chain` array becomes very large
4. This increases the JWT size, which could trigger header budget limits (Story 2.5)
5. Parsing a very deep chain could cause stack overflow or memory exhaustion

**Impact:** Stack exhaustion / DoS via deeply nested delegation chains.

**Fix:**
1. Add maximum chain depth limit (e.g., 10 levels) in Story 6.1
2. Reject token exchange requests where the `act.chain` would exceed the limit
3. Add security regression test: "Given a nested delegation chain with 100 levels, assert the handler rejects the request"
4. Add to Story 6.1 acceptance criteria: "act.chain is truncated to a maximum depth (e.g., 10 levels) to prevent stack overflow or excessive memory usage"

---

## HOLE 21: Fallback Cache Single-Flight Not Specified

**Target:** Story 4.3 (Selective Online Fallback)

The wiki mentions "Single-flight pattern: only one request hits authz-core for a given cache key; others wait for the result." But this is not specified in Story 4.3's implementation details or acceptance criteria.

**Exploit Path (Detailed):**
1. The fallback cache TTL expires for a popular endpoint (e.g., `PUT /api/v1/identity/preferences`)
2. 1000 requests arrive simultaneously
3. Without single-flight, all 1000 requests call authz-core simultaneously (thundering herd)
4. authz-core becomes overwhelmed, causing slow responses or timeouts
5. Result: **cache miss thundering herd DoS**

**Impact:** Cache miss thundering herd can cause service degradation that mimics an authz-core outage. This is particularly dangerous because:
- The load is concentrated on authz-core (which is already the bottleneck)
- The thundering herd can last for the entire TTL window (5-30 seconds)
- After the TTL expires again, the thundering herd repeats

**Fix:**
1. Implement single-flight pattern in Story 4.3: only one request hits authz-core for a given cache key; others wait for the result
2. Use a mutex or semaphore keyed by the cache hash to serialize authz-core calls
3. Add to Story 4.3 acceptance criteria: "When cache expires, only one request hits authz-core for a given cache key; others wait for the result"
4. Add a "stale cache" optimization: serve from stale cache (slightly expired) while the fresh cache is being populated
5. Add monitoring: alert on "authz-core QPS spike" (indicates possible thundering herd)

---

## HOLE 22: `SesameAuthzClaims.permissions` Is a Raw `Vec<String>` With No Signature

**Target:** Story 2.2 (TokenClaims structs)

`SesameAuthzClaims` in Story 2.2 has `pub permissions: Vec<String>` — a plain array of permission strings, serialized as part of the JWT payload. While the JWT signature protects the entire payload, there is NO per-field signature.

**Exploit Path (Detailed):**
1. The JWT is valid (correct signature)
2. But `sx.permissions` is just a list of strings in the payload — there's no individual signature on the permissions
3. If an attacker can inject into the JWT payload (e.g., through a JWT library vulnerability, or through a side-channel that allows payload modification without invalidating the signature), they can add arbitrary permissions
4. The JWT signature does protect against this, but the point is: `sx.permissions` is a LIST, not a HASH. If the JWT signature is somehow broken (future crypto attack, implementation bug), the permissions are readable and forgeable

**Why this matters:** The `entitlements_hash` (Story 2.3) is designed to prevent cache poisoning (Hole 1). But `sx.permissions` has NO equivalent protection. If an attacker can forge a JWT (even temporarily), they can inject arbitrary permissions.

**Impact:** Trust boundary illusion. The `Vec<String>` gives the appearance of structured data but has no internal integrity check. If the JWT signature is compromised, the permissions are readable and forgeable.

**Fix:**
1. Consider replacing `sx.permissions` with `sx.permissions_hash` (a hash of the permissions array) — similar to `entitlements_hash`
2. If permissions must remain as a list, add a `permissions_hash` field that covers the `sx.permissions` array
3. Handlers that check permissions should verify the hash before trusting the list
4. Add to Story 2.2: "If permissions are embedded in the JWT, include a hash of the permissions array for integrity verification"

---

## Positive Findings (Design Strengths)

- **PII removal (Story 2.3):** Correct and follows OWASP recommendations
- **Entitlements reference pattern (Story 2.3):** Right approach for large permission sets (if hash verification is added)
- **Token family-based reuse detection (Story 3.2):** Well-reasoned approach to the "tear" scenario
- **Shadow decision migration (Story 9.4):** Correct approach for safely migrating to JWT common-path
- **Decision matrix by endpoint type (Story 4.4):** Correctly distinguishes trust-creation from trust-evaluation routes
- **Three-layer revocation model (TTL + version + jti denylist):** Properly designed
- **Audit logging format (Story 8.3):** Comprehensive and PII-free
- **NGINX header budget analysis (Story 2.5):** Thorough and realistic
- **MFA type strength enforcement (Story 6.3):** Correctly enforces TOTP/WebAuthn for critical actions
- **Token versioning (Story 5.1-5.5):** Well-designed mechanism for privilege invalidation
- **Single-flight fallback (Wiki):** Correct approach to cache miss thundering herd

---

## Recommended Fix Priority

| Priority | Hole | Effort | Blocker For | Related F-Code |
|----------|------|--------|-------------|----------------|
| P0 | #1 Entitlements hash verification | Low | Story 4.3 | F-007 |
| P0 | #3 DPoP token binding | Medium | Epic 8 | F-004 |
| P0 | #10 Rate limiting on auth endpoints | Low | All auth stories | F-009 |
| P0 | #16 extract_jti signature validation | Low | Story 8.4 | F-002 |
| P1 | #2 Version check fail-open | Low | Story 5.2 | F-013 |
| P1 | #4 Refresh token binding | Medium | Story 3.1 | F-015 |
| P1 | #5 Tenant ID validation | Low | Story 4.2 | — |
| P1 | #17 Token exchange audience merging | Low | Story 6.1 | F-003, F-012 |
| P2 | #6 Login endpoint rate limiting | Low | Story 4.4 | — |
| P2 | #7 Permission cross-check for high-risk routes | Medium | Story 4.4 | — |
| P2 | #8 Version check for all routes | Low | Story 5.2 | — |
| P2 | #9 Full session invalidation on MFA step-up | Low | Story 6.3 | F-006 |
| P2 | #18 MFA step-up token version bump | Low | Story 5.1 | F-006 |
| P3 | #11 User enumeration prevention | Low | Story 4.4 | — |
| P3 | #12 authz-core degradation mode | Medium | Epic 4 | — |
| P3 | #13 JWKS information leakage | Low | Story 1.2 | F-009 |
| P3 | #14 Key compromise detection | Medium | Story 1.1 | — |
| P3 | #15 Token exchange org validation | Low | Story 6.1 | — |
| P3 | #19 Password reset endpoint | Low | All stories | — |
| P3 | #20 act.chain depth bound | Low | Story 6.1 | — |
| P3 | #21 Fallback cache single-flight | Low | Story 4.3 | — |
| P3 | #22 permissions hash verification | Low | Story 2.2 | — |

---

## Cross-Cutting Attack Scenarios

### Scenario A: The "Everything at Once" Attack

An attacker with a stolen access token combines multiple holes:
1. Uses the stolen token from a different device (Hole 3: no token binding)
2. The token has stale permissions (Hole 8: version check only for elevated risk)
3. The attacker changes `X-Tenant-ID` to access another tenant's data (Hole 5: tenant validation gap)
4. The attacker makes high-consequence requests (Hole 7: permissions trusted from JWT)
5. The attacker floods endpoints to slow down detection (Hole 10: no rate limiting)

**Result:** Complete tenant-bleed data exfiltration with no detection for 5 minutes (the token TTL).

### Scenario B: The "Login Flood + authz-core DoS" Attack

An attacker floods the login endpoint with valid credentials:
1. Each login triggers authz-core `/principal/effective` (Hole 6: no login rate limiting)
2. authz-core becomes overwhelmed
3. All jwt-with-fallback and online-only routes fail (Hole 12: authz-core SPOF)
4. Only jwt-only routes work (graceful degradation)
5. The attacker can still read data for existing sessions but cannot write or admin

**Result:** Partial outage — writes and admin actions are blocked, but reads continue.

### Scenario C: The "Token Exchange Privilege Escalation" Attack

An attacker with a platform admin token in org A:
1. Initiates token exchange with an org B user's token as subject
2. Gets a new token with `act.sub = platform_admin` (org A) and `sub = org B user`
3. The new token can act as org B's user but with org A's admin privileges (Hole 15: cross-org delegation)
4. The attacker can now access org B's data using org A's admin identity

**Result:** Cross-org privilege escalation through token exchange.

---

## Files Referenced

- `docs/Epics/INDEX.md`
- `docs/Epics/01-asymmetric-jwks/JWT.md`
- `docs/Epics/01-asymmetric-jwks/stories/story-1.1.md` through `story-1.4.md`
- `docs/Epics/02-claims-schema-evolution/claims.md`
- `docs/Epics/02-claims-schema-evolution/stories/story-2.2.md`
- `docs/Epics/03-token-lifecycle/tokens.md`
- `docs/Epics/03-token-lifecycle/stories/story-3.1.md`
- `docs/Epics/04-hybrid-authz-model/hybrid.md`
- `docs/Epics/04-hybrid-authz-model/stories/story-4.3.md`
- `docs/Epics/04-hybrid-authz-model/stories/story-4.4.md`
- `docs/Epics/05-token-versioning/versioning.md`
- `docs/Epics/06-delegation-act/stories/story-6.1.md`
- `docs/Epics/06-delegation-act/stories/story-6.3.md`
- `docs/Epics/08-security-hardening/security.md`
- `docs/Epics/security-assessment-JWT-authz.md`
- `docs/Sesame-idam_authorisation_load_mitigation_with_JWT_claims.md`
- `docs/llmwiki/topics/topic-hybrid-authz.md`
- `docs/llmwiki/topics/topic-jwt-schema.md`
- `docs/llmwiki/topics/topic-token-versioning.md`
- `docs/llmwiki/topics/topic-tenancy-model.md`
- `docs/llmwiki/topics/topic-delegation.md`
- `docs/llmwiki/topics/topic-mfa.md`
- `docs/llmwiki/topics/topic-login-flow.md`
- `docs/llmwiki/topics/topic-authorization-flow.md`
- `docs/llmwiki/topics/topic-rls-bridge.md`
- `AGENTS.md`
- `microservices/idam/authz-core/`
- `microservices/idam/identity-login-service/`
- `microservices/idam/identity-session-service/`
- `docs/Epics/09-observability/observability.md`

---

## HOLE 23: Structured Logging Observability Data Leakage (NEW — from Epic 9)

**Target:** Story 9.1 (JWT Validation Spans) + Story 9.5 (Token Lifecycle Spans) + Story 9.6 (Structured JWT Logging)

The observability stories create OTEL spans and structured logs for JWT operations. These spans/logs contain user context (`user_id`, `tenant_id`, `subject`, `session_id`, `token_version`, `decision_source`, `actor_subject`) that are visible to anyone with access to the observability stack (Jaeger/Loki/Grafana).

**Exploit Path (observability data leak):**
1. Attacker gains access to Jaeger/Loki (via compromised service account, misconfigured RBAC, or exploitation of the observability stack)
2. Attacker queries for `jwt_validation` spans with `user_id`, `tenant_id`, `decision_source` fields
3. Attacker extracts all user IDs, tenant IDs, authorization decisions, and delegation chains from the spans/logs
4. Attacker builds a complete map of: which users exist, which tenants exist, which routes each user accesses, which users have delegation chains
5. Result: **User/tenant/authorization system reconnaissance from observability data**

**Why this is critical:** Observability systems (Jaeger, Loki, Grafana) are often less protected than the application itself. They may have weaker RBAC, less monitoring, or be accessible from the same network segment as the application. The spans/logs contain enough information for an attacker to plan targeted attacks.

**Additional risk: Log field injection (HACK-961):** If the structured logger merges JWT claims into log entries at the top level, an attacker can forge a JWT with claims that match log field names (e.g., `level`, `event`, `service`) to manipulate log metadata or misclassify log levels.

**Fix:**
1. NEVER merge JWT claims into structured log entries at the top level — use a nested `claims` object
2. ALL log fields MUST be set explicitly by the middleware, never from JWT claims
3. Do NOT record `user_id`, `tenant_id`, or PII in spans — only in secure audit logs with restricted access
4. Log access MUST be restricted via RBAC on Loki/Grafana (separate from application access)
5. Add a validation step: "Verify that no JWT claim name matches a log field name"

---

## HOLE 24: Alert System Cannot Detect Forged Tokens with Valid Signatures (NEW — from Story 9.7)

**Target:** Story 9.7 (Alerting Configuration)

The alerting system detects INVALID tokens (expired, revoked, wrong issuer, etc.) but CANNOT detect tokens with VALID signatures that were FORGED by an attacker with the signing key.

**Exploit Path (forged token with valid signature):**
1. Attacker obtains the JWT signing key (via key confusion attack if HS256 is still supported, via memory dump, or via insider threat)
2. Attacker forges a token with elevated permissions (`role: admin`, `sx.permissions = ["admin:all"]`)
3. The forged token PASSES all validation (valid signature, valid issuer, valid expiration)
4. The alert system does NOT trigger because the token is "valid"
5. The attacker has full access for the entire token TTL (5 minutes) or until the signing key is detected as compromised

**This is a fundamental limitation:** No alert system can detect a valid token that was forged — by definition, a valid token passes all validation checks. The only defense is to prevent key compromise (use RS256, not HS256) and detect key compromise quickly (key rotation, monitoring).

**Fix:**
1. Ensure HS256 is completely removed (Story 1.4) — no fallback to symmetric signing
2. Implement key rotation monitoring: alert if the signing key has not been rotated in the configured interval
3. The PRIMARY defense against forged tokens is KEY MANAGEMENT, not alerting

---

## HOLE 25: Shadow Mode Can Be Weaponized as an Authorization Oracle (NEW — from Story 9.4 + Story 9.7)

**Target:** Story 9.4 (Shadow Decision Observability) + Story 9.7 (Alerting Configuration)

During migration, shadow mode is enabled and compares JWT decisions against online authz-core decisions. The `shadow_mismatch` alert reveals which routes have jwt-with-fallback authorization AND which JWT claims are incomplete.

**Exploit Path (shadow mismatch as oracle):**
1. Attacker has access to the alerting channel (Slack #idam-alerts, PagerDuty, or Loki logs)
2. Attacker sends requests to different routes
3. For routes where shadow mismatch occurs (JWT claim diverges from online), an alert fires
4. For routes with no shadow check (jwt-only), no alert fires
5. Result: **The attacker maps which routes use which authorization path**

**Additionally:** Shadow mode doubles the authz-core load (Story 9.4 trade-off). If shadow mode is left enabled in production, an attacker can exploit this to create a DoS via authz-core overload.

**Fix:**
1. Shadow mode MUST be disabled in production — enforce via startup check that blocks startup if enabled
2. Add a watchdog: detect if shadow mode is still enabled after migration completion
3. Shadow mismatch alerts in Slack must NOT include route-specific details
4. If shadow mode is accidentally enabled, add a rate limit on authz-core calls for shadow mode

---

## HOLE 26: Alert Fatigue as a Denial-of-Service (NEW — from Story 9.7)

**Target:** Story 9.7 (Alerting Configuration)

The alert system generates WARNING alerts for `jwt_validation_failed` when rate exceeds 5/min. An attacker can deliberately trigger WARNING alerts to create alert fatigue, causing the on-call team to ignore real CRITICAL alerts.

**Exploit Path (alert fatigue DoS):**
1. Attacker floods the system with invalid JWTs to trigger `jwt_validation_failed` alerts (> 5/min)
2. Slack notifications flood #idam-alerts
3. Meanwhile, the attacker sends a token reuse detection event (CRITICAL — actual token theft)
4. PagerDuty fires, but the on-call team is overwhelmed with false warnings
5. Result: **The real alert is dismissed as "just another false alarm"**

**Fix:**
1. Add a "quiet period" to WARNING alerts (e.g., 15 minutes between consecutive WARNING alerts for the same event type)
2. CRITICAL alerts (reuse_detected, rotation_failure, etc.) should ALWAYS fire immediately, regardless of recent WARNING volume
3. Add a `alert_suppression_total{event, reason: "quiet_period"}` metric to track suppressed alerts

---

## Updated Recommended Fix Priority

Add to the table above (Priority column):

|| Priority | Hole | Effort | Blocker For | Related F-Code |
|----------|------|--------|-------------|-------------|----------------|
| P1 | #23 Observability data leakage | Low | Story 9.1-9.6 | All | — |
| P1 | #24 Alert system blind to forged tokens | Low | Story 9.7 | All | — |
| P1 | #25 Shadow mode as authorization oracle | Low | Story 9.4 | All | — |
| P2 | #26 Alert fatigue DoS | Low | Story 9.7 | All | — |

---

## Summary of Additional Observability/Alerting Holes (Added 2026-05-16)

|| # | Hole | Impact | Difficulty | Target Stories |
|---|------|--------|----------|-------------|----------------|
| 23 | Structured logging data leakage | User/tenant/authorization reconnaissance | Easy | 9.1, 9.5, 9.6 |
| 24 | Alert system blind to forged tokens | Unauthorized access with forged tokens | Trivial | 9.7 |
| 25 | Shadow mode as authorization oracle | Route mapping, claim gap identification | Easy | 9.4, 9.7 |
| 26 | Alert fatigue DoS | Missed real security alerts | Trivial | 9.7 |

These holes are ALL NEW — they were not identified in the original 22-hole assessment. They were discovered during the malicious actor analysis of the observability and alerting stories (Epic 9).
