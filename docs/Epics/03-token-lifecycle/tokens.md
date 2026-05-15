# Epic 3: Token Lifecycle & Refresh Rotation

## Summary

Implement rotating refresh tokens with reuse detection, short-lived access tokens (5-15 minutes), and token exchange flows (RFC 8693). Replace the current refresh token model with a rotating token family stored in Redis, where each refresh is validated, the old token is blacklisted for the family TTL, and a new refresh token is issued.

## Why This Epic Is Needed

The JWT document emphasizes rotating refresh tokens as the primary revocation mechanism. Without rotation, a stolen refresh token can be replayed indefinitely until it expires. Without reuse detection, a stolen refresh token can be replayed by both the legitimate user and the attacker. The current implementation stores session tokens and refresh tokens in both PG and Redis but doesn't explicitly implement rotation or family-based reuse detection.

## Current State

- Refresh tokens stored in Redis + PG
- Session tokens stored in both PG and Redis
- `redis.rs` module handles refresh-token metadata, per-user session sets, and jti blacklist
- Token issuance at login calls authz-core `/principal/effective` once
- JWT document recommends 5-15 minute access token TTL, 7-30 day refresh tokens

## Stories

- [ ] Story 3.1: Implement refresh token rotation
  - On every `/refresh` call: validate old refresh token against Redis
  - Invalidate old token, issue new refresh token with new `jti`
  - Store the old `jti` in denylist cache for family TTL (e.g., 24 hours) to detect reuse
  - Issue new access token with fresh claims (if token version has changed)

- [ ] Story 3.2: Implement refresh token family / reuse detection
  - Group refresh tokens into families (one family per active user session)
  - When a token is used, mark the entire family for reuse detection
  - If the same family token is used twice, invalidate all tokens and require re-auth
  - Prevent "tear" scenario where both attacker and legitimate user have the same token

- [ ] Story 3.3: Configure access token TTL
  - Set access token TTL to 5 minutes for normal users
  - Set access token TTL to 1-5 minutes for admin/high-privilege tokens
  - Configurable via environment variable per role tier

- [ ] Story 3.4: Implement RFC 8693 token exchange
  - `POST /token` with `grant_type=urn:ietf:params:oauth:grant-type:token-exchange`
  - Accept a valid access token as a subject token
  - Issue a new access token with delegated claims (`act` claim)
  - Used for service-to-service delegation and support tool impersonation

- [ ] Story 3.5: Implement rotating refresh token logout
  - On `/auth/logout`: invalidate the entire token family in Redis
  - Invalidate all access tokens from the denylist cache
  - Return to the client (or ignore for security — don't confirm logout to prevent enumeration)

## OpenAPI Changes Needed

- `/auth/refresh` response schema: ensure it documents the rotating refresh token behavior
- Token exchange endpoint needs to be added to the login-service spec (new endpoint)
- Logout response: consider whether to return a body or silent 204

## Design Doc Changes Needed

- `design-doc.md`: Update session management section with rotation details
- `design-doc.md`: Add token exchange flow diagram
- `service-topology-design.md`: Update per-request cost model for identity services with rotation
- Wiki: Update `topics/topic-login-flow.md` with rotation details
- Wiki: Create `topics/topic-token-lifecycle.md` (new)

## Gaps in the JWT Document

- Does not specify the refresh token family data structure (how to group tokens into families in Redis).
- Does not address what happens when a user is in multiple sessions (e.g., logged in on web and mobile). Should logout-all invalidate one session or all?
- Does not specify the replay detection window (how long to keep old `jti` in denylist).
- Does not address the "logout-all" use case explicitly.

## Dependencies

- Depends on Epic 1 (JWKS) for token validation infrastructure
- Requires Redis infrastructure already in place (it is)
