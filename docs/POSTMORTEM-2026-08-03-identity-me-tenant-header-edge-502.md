# Postmortem: `/identity/me` 502 after public-edge dogfood (stripped `X-Tenant-ID`)

Date of incident: 2026-08-03  
Status: FIX IN FLIGHT (Option A — JWT-bound tenant). Blameless.  
Repos involved: sesame-idam (`identity-session-service`, `sesame-idam-api-edge`),
hauliage/loadlinker BFF + frontend.  
Also filed in: hauliage
`docs/postmortems/postmortem-identity-me-tenant-header-edge-502-2026-08-03.md`.

## Summary

After Hauliage dogfood was pointed at Sesame’s **public** API
(`https://api.sesameidentity.dev.local/idam/v1`), login succeeded and
`GET /api/v1/organizations/me` returned **200**, but
`GET /api/v1/identity/users/me` returned **502** from the BFF.

The BFF correctly called Sesame `GET /idam/v1/identity/me` with a Bearer
access token **and** `X-Tenant-ID: hauliage`. The API edge **removed**
`X-Tenant-ID` (by design). Session still **required** that header in OpenAPI
codegen, so the request failed before handler logic with
`Missing required parameter 'X-Tenant-ID'`. The BFF mapped the upstream
failure to **502** `identity_unavailable`.

No credentials were stolen. Impact was availability of the post-login
profile path for north–south consumers.

## Timeline (all 2026)

| When | What |
|---|---|
| Pre-JWT / east–west era | Tenant consumers (Hauliage) called Sesame **in-cluster** with `X-Tenant-ID` on every hop. That header was the practical tenancy signal when tokens were incomplete, untrusted, or not yet verified end-to-end. |
| Jul 25 | FINDING: org-mgmt preferred `X-Tenant-ID` over the JWT `tenant_id` claim (cross-tenant with a valid token). Fix tracked as task 48. |
| Jul–Aug | API edge ships `stripTenantHeader: true` so forgeable ambient tenancy cannot override a signed claim on the north–south path. Correct mitigation **if** services derive tenant from JWT. |
| Epic 15 | Portable consumer contract: tenant after auth is **token-bound**; callers must not select tenancy with `X-Tenant-ID`. |
| Epic 16 dogfood | Hauliage BFF Sesame bases move to `https://api.…/idam/v1` (public edge + CA bundle). Login works; profile fetch breaks. |
| Aug 3 | Reproduce with a real login token: public `GET …/identity/me` → HTTP 500 / missing `X-Tenant-ID`. In-cluster session without header → 401 (auth), proving routing is fine; the required header is the defect. |
| Aug 3 | Option A: OpenAPI makes `X-Tenant-ID` optional on `/identity/me` and `/identity/userinfo`; `authenticated_principal` derives tenant from JWT; header kept only as a mismatch check. |

## Why it was that way before JWTs worked correctly

1. **Pre-authentication idiom leaked into post-authentication APIs.**  
   Login/register legitimately need a tenant selector (`X-Tenant-ID` or
   `client_id`) because there is no access token yet. Session profile
   endpoints copied the same required header even though they already
   receive a Bearer JWT with `tenant_id`.

2. **East–west dogfood hid the mismatch.**  
   While Hauliage talked to `identity-session-service` on the cluster
   network, the edge never stripped the header. Required-header +
   BFF-supplied slug always agreed. The broken combination only appeared
   when the consumer moved to `api.` (north–south).

3. **Edge mitigation landed ahead of the service fix.**  
   `stripTenantHeader` correctly addressed FINDING-2026-07-25 for the
   public surface, but session OpenAPI still treated the stripped header
   as mandatory. The mitigation and the handler contract were one release
   out of sync.

4. **Incomplete JWT trust historically justified the header.**  
   Until signing, JWKS casing, issuer/audience, and BFF verification were
   reliable, teams treated `X-Tenant-ID` as a belt-and-suspenders routing
   key. That was reasonable temporarily and wrong once tokens became the
   authority.

## Root cause (layered)

1. **OpenAPI / codegen:** `GET`/`PATCH /identity/me` (and userinfo) marked
   `X-Tenant-ID` `required: true` → generated `TryFrom` failed if absent.
2. **Handler:** `authenticated_principal` required a non-optional header
   string and compared it to the claim (reject-on-mismatch), but could not
   run if codegen rejected the request first.
3. **Edge:** `sesame-idam-api-edge` `stripTenantHeader: true` removed the
   header on `/idam/v1/identity/`.
4. **Consumer mapping:** Hauliage BFF maps Sesame transport/upstream errors
   on this path to **502**, which obscured the Sesame 4xx/5xx body in the
   browser.

## Proposed solution and fix (Option A)

**After authentication, tenant comes only from the validated JWT
`tenant_id` claim.** Optional `X-Tenant-ID` is a consistency check: if
present and non-empty it must match; if absent (edge strip), proceed.

Implemented in sesame-idam:

- OpenAPI: `X-Tenant-ID` `required: false` on `/identity/me` and
  `/identity/userinfo`; descriptions updated to JWT-bound tenancy.
- Regen: identity-session-service gen handlers accept optional header.
- `authenticated_principal`: same pattern as identity-login-service
  (claim authoritative; header optional mismatch check).
- Tests: unit coverage for claim-only / match / mismatch; BDD scenario
  for missing header.

**Not chosen:** rolling back edge strip (reopens header override) or
routing the BFF east–west only (abandons public-contract dogfood).

Edge `stripTenantHeader` **stays**. It is correct once handlers do not
require the header.

## What went well

- Login and org paths narrowed the blast radius quickly.
- Direct curl against `api.` with a real token made the missing-header
  error unambiguous.
- Prior FINDING + Epic 15 docs already named the correct end state;
  this incident was unfinished migration, not a new design debate.

## Action items

| # | Action | Owner | Status |
|---|---|---|---|
| 1 | Ship Option A for session `/identity/me` (+ userinfo); redeploy session | sesame-idam | IN PROGRESS |
| 2 | Sweep other bearer-bound session routes still requiring `X-Tenant-ID` under stripped paths | sesame-idam | OPEN |
| 3 | Finish task 48 everywhere (claim-only / reject on disagree) | sesame-idam | OPEN |
| 4 | Optional: BFF surface upstream Sesame body/status in `identity_unavailable` for faster triage | hauliage | OPEN |
| 5 | North–south smoke: login → `GET /api/v1/identity/users/me` 200 after deploy | both | OPEN |

## Verification

```bash
# After session image rolls with Option A:
TOKEN=…  # access_token from password login
curl -sS -o /tmp/me.json -w '%{http_code}\n' \
  -H "Authorization: Bearer $TOKEN" \
  https://api.sesameidentity.dev.local/idam/v1/identity/me
# expect: 200 and profile JSON — no X-Tenant-ID required

# Browser: loadlinker login → Network: /api/v1/identity/users/me → 200
```

## Decision affirmed

**Public edges may strip caller-selected tenancy. Authenticated Sesame
APIs must not require what the edge removes.** JWT `tenant_id` is the
tenant authority after login; `X-Tenant-ID` remains for pre-auth flows
only (or as a non-authoritative mismatch check).
