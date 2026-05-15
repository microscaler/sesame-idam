# Epic 6: Delegation & Actor Claims

## Summary

Implement RFC 8693 `act` claim support for delegation, enabling service-to-service impersonation and support tool flows where the actor must see both the subject and the current actor. Add token exchange that converts an `act` token into a token containing the `act` claim.

## Why This Epic Is Needed

The JWT document emphasizes that "top-level claims plus the current actor are what matter for access control; deeper nested actors are audit information, not decision inputs." Without delegation support, there is no way to implement support tool impersonation, platform automation "on behalf of user" flows, or service-to-service token exchange. RFC 8693 provides the standards-track mechanism.

## Current State

- No `act` claim support
- No token exchange endpoint
- No delegation infrastructure
- Generated runtime contains basic JWT support but no RFC 8693 handling
- No impersonation flow (the topology mentions "impersonation" as a high-frequency identity-session-service capability, but it's not detailed)

## Stories

- [ ] Story 6.1: Implement RFC 8693 token exchange
  - Accept a valid access token as a subject token
  - Return a new token containing `act: {sub: "svc_support_tool", ...}`
  - The new token retains the original subject's claims plus the actor
  - Rate limit and log all token exchange requests

- [ ] Story 6.2: Implement `act` claim validation
  - Validate `act.sub` exists when present
  - Validate that the actor has permission to act on behalf of the subject
  - Document that nested actors beyond the top-level are audit-only
  - Enforce that `act` does not accidentally inherit more privilege than intended

- [ ] Story 6.3: Implement support tool impersonation flow
  - Platform admin initiates impersonation: POST /auth/impersonate {user_id, actor_id}
  - Returns a delegated token with `act` claim pointing to the admin's identity
  - Consuming API sees both the impersonated user and the admin actor
  - Audit log records who impersonated whom

- [ ] Story 6.4: Implement "act as" API key delegation
  - API keys can have a `delegated_to` field (nullable)
  - When an API key is used with delegation, the resulting token contains the `act` claim
  - Allows M2M services to act on behalf of individual users
  - Revoking the API key revokes the delegation

## OpenAPI Changes Needed

- New endpoint: `POST /api/v1/identity/token/exchange` (RFC 8693 token exchange)
- New endpoint: `POST /api/v1/identity/auth/impersonate` (support tool impersonation)
- API Key schema: Add optional `delegated_to` field

## Design Doc Changes Needed

- `design-doc.md`: Add RFC 8693 delegation section
- `design-doc.md`: Add token exchange flow diagram
- `design-doc.md`: Document the `act` claim in the JWT enrichment section
- Wiki: Create `topics/topic-delegation.md` (new)
- Wiki: Update `topics/topic-login-flow.md` to mention token exchange

## Gaps in the JWT Document

- Does not specify the actor_token format. The document references `act.sub` but doesn't define what other fields should be in the `act` object (e.g., `act.tenant`, `act.portal`, `act.roles`).
- Does not address the case where a user is impersonated by multiple actors in sequence. Should there be a chain of actors or just the current one?
- Does not address the security implications of delegation: how to limit which users can be impersonated, which services can perform impersonation.
- The document's Rust example shows `act: Option<ActorClaim>` with only `sub`. This is insufficient -- it should also carry `tenant` and `portal` to match the subject's context.

## Dependencies

- Depends on Epic 1 (JWKS) and Epic 2 (Claims Schema) for token validation infrastructure
- Intersects with Epic 4 (Hybrid Authz) for `act`-aware route classification
- Intersects with Epic 3 (Token Lifecycle) for delegation token expiration
