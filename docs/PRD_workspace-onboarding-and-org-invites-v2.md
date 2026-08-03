# PRD: Workspace Onboarding, Org Invites & Membership (Platform v2)

| Field | Value |
|-------|--------|
| **Version** | `v2026-08-03` |
| **Status** | Draft — iterate before implementation freeze |
| **Owner** | Sesame-IDAM (platform) |
| **Paired consumer PRD** | [`../hauliage/docs/PRD_loadlinker-workspace-onboarding-and-team-v2.md`](../../hauliage/docs/PRD_loadlinker-workspace-onboarding-and-team-v2.md) |
| **Supersedes** | Hauliage `PRD_account-first-onboarding.md` (product narrative); Sesame `topic-account-first-onboarding-checkpoint.md` (checkpoint) |
| **Normative companions** | [ADR-002](./ADR-002-tenant-consumer-idam-api-boundary.md), [transport-policy-v1](./standards-first-oidc/transport-policy-v1.md), Epic 15 portable consumer contract |
| **Evidence** | [POSTMORTEM-2026-08-03-identity-me-tenant-header-edge-502](./POSTMORTEM-2026-08-03-identity-me-tenant-header-edge-502.md) |

---

## 1. Executive summary

Sesame is the system of record for **identity, organizations, memberships, and invitations**. Product BFFs (Hauliage Loadlinker first) orchestrate domain profile creation; they must not own invite tokens or membership rows.

JWT authentication is now functional in dogfood. That makes three platform rules mandatory:

1. **After authentication, tenant is claim-bound** (`tenant_id` on the access token). Callers must not select tenancy with `X-Tenant-ID` on bearer routes. Public edges may strip that header.
2. **Org context is claim-bound** (`org_id` when the user has an active membership). Login/register without an org yields a valid token **without** `org_id` for onboarding-only use.
3. **Invites are Sesame-owned**: persist `org_invites`, send magic-link email, accept creates `org_memberships` and enables `org_id` re-issue.

This PRD replaces the pre-JWT “header + bridge table” product story with a contract any tenant consumer can implement.

---

## 2. Problem

### 2.1 Product gap

New users must be able to:

- Register an account without an organization.
- Create a workspace **or** accept an invite.
- Invite colleagues by email and see real member identities on team lists.

### 2.2 Live platform defects (2026-08-03 dogfood)

| Defect | Symptom | Cause |
|--------|---------|--------|
| Bearer routes require `X-Tenant-ID` | Public edge strip → 500/502 | OpenAPI/handler still header-required; edge correctly strips forgeable tenancy |
| Member emails are placeholders | `user-{uuid}` in list responses | `SELECT email FROM users` fails RLS (`sesame_current_tenant_id()` unset) |
| Invite create fails north–south | Consumer BFF 502 | `invite_user_to_org` reads tenant from header only |
| Invite email not sent | Mailpit has OTP/reset but no org invites | Invite path persists + logs token; no SMTP |

Older docs claimed account-first “verified” while these gaps remained. This PRD is the single backlog for closing them on the **provider** side.

---

## 3. Goals

1. Register = identity only (no org, no `org_id` claim).
2. Create organization → owner membership → access token includes `org_id`.
3. Invite by email → Sesame `org_invites` + outbound email + token in API response.
4. Accept invite → membership + refreshed tokens with `org_id`.
5. List org members with **real emails** (and names when profile exists), never synthetic `user-{uuid}` placeholders when the user row exists.
6. Bearer APIs work through the public API edge with **no** required `X-Tenant-ID`.
7. Demo seeds for tenant `hauliage` remain first-class fixtures for dogfood.

## 4. Non-goals

| Item | Rationale |
|------|-----------|
| Email-domain auto-org | Rejected; freemail / accidental merges |
| SMS invite channel (v1) | Email magic-link only; SMS OTP remains ADR-009 |
| Multi-org switcher UI | v1: single active `org_id` in JWT; switch API may exist |
| Hauliage domain profile schema | Consumer PRD / company service |
| Caller-selected tenancy on bearer routes | Forbidden by Epic 15 / edge strip |
| Webhooks for org.created (v1) | Sync BFF chain remains MVP; webhooks are follow-on (ADR-002 S3) |

---

## 5. Actors and tenancy

| Concept | Meaning |
|---------|---------|
| **Tenant** | Hard isolation boundary (`hauliage`, …). Maps to SaaS customer partition. |
| **Application / RP** | OAuth client within a tenant (e.g. `hauliage-web`). |
| **Organization** | Workspace inside a tenant (shipper or haulier company in product terms). |
| **User** | Identity scoped to one tenant. |
| **Membership** | User ↔ org + role (`owner`, `admin`, `member`, product-mapped roles). |
| **Invite** | Pending email → org + role + opaque token + expiry. |

### 5.1 Pre-auth vs bearer

| Phase | Tenant resolution |
|-------|-------------------|
| Login / register / OTP (no access token) | Registered client and/or optional legacy `X-Tenant-ID` hint that must match client tenant |
| Any Bearer-authenticated route | Validated JWT `tenant_id` (and `org_id` when required). Optional `X-Tenant-ID`: if present and non-empty, **must match** claim or **401**; if absent, proceed |

---

## 6. User journeys (platform)

### 6.1 Register (identity only)

1. Client → `POST /idam/v1/auth/register` (tenant via client / pre-auth policy).
2. Sesame creates `users` row; issues access + refresh tokens.
3. Access token has `sub`, `tenant_id`; **no** `org_id`.
4. Client directs user to product onboarding (out of Sesame UI scope).

### 6.2 Login without membership

1. `POST /idam/v1/auth/login` succeeds.
2. Token has `tenant_id`, no `org_id`.
3. Org-scoped APIs that require membership return 403/404 as specified; onboarding APIs remain available.

### 6.3 Create organization

1. Authenticated user (Bearer) → `POST /idam/v1/organizations` (or tenant-consumer equivalent) with name (+ optional metadata).
2. Sesame creates `organizations` + `org_memberships` (caller = `owner`).
3. Client calls `POST /idam/v1/sessions/active-organization` (or login re-issue) → new access token with `org_id`.
4. Product BFF may then create a domain profile keyed by the same UUID (consumer concern).

### 6.4 Invite member

1. Org admin/owner → `POST /idam/v1/organizations/{org_id}/invitations` with `{ email, role }`.
2. Sesame inserts `org_invites` (token, expiry ≥ 7 days default).
3. Sesame **sends** invite email (SMTP) containing magic link URL template configured per environment.
4. Response includes `invite_id` and `invite_token` (required for automated E2E; email is human path).

### 6.5 Accept invite

1. Invitee registers or logs in (email must match invite, case-insensitive).
2. `POST /idam/v1/invitations/accept` with `{ token }` + Bearer.
3. Sesame creates/activates membership; marks invite accepted.
4. Client obtains refreshed tokens with `org_id` via active-organization / accept response contract.

### 6.6 List members / remove / revoke

1. `GET /organizations/{org_id}/users` returns paginated members with real `email`, `user_id`, `role`, `created_at`.
2. Remove member and revoke pending invite are org-admin operations; tenant from JWT.

```mermaid
flowchart LR
  register[Register] --> tokenNoOrg[JWT_no_org_id]
  tokenNoOrg --> createOrg[Create_org]
  tokenNoOrg --> acceptInv[Accept_invite]
  createOrg --> tokenOrg[JWT_with_org_id]
  acceptInv --> tokenOrg
  tokenOrg --> invite[Invite_by_email]
  invite --> email[SMTP_magic_link]
  invite --> e2eToken[invite_token_in_response]
  email --> acceptInv
  e2eToken --> acceptInv
```

---

## 7. Normative API surface

Prefer **tenant-consumer** OpenAPI for product teams; org-mgmt remains the implementing service where routes already live.

| Operation | Method / path (illustrative `/idam/v1`) | Auth | Tenant |
|-----------|----------------------------------------|------|--------|
| Register | `POST /auth/register` | none / pre-auth | client / hint |
| Login | `POST /auth/login` | none / pre-auth | client / hint |
| Current user profile | `GET /identity/me` | Bearer | JWT only |
| Create org | `POST /organizations` | Bearer | JWT |
| Set active org | `POST /sessions/active-organization` | Bearer | JWT |
| List members | `GET /organizations/{org_id}/users` | Bearer | JWT |
| Invite | `POST /organizations/{org_id}/invitations` | Bearer | JWT |
| Preview invite | `GET /invitations/preview?token=` | public or light auth | N/A (token-bound) |
| Accept invite | `POST /invitations/accept` | Bearer | JWT |
| Revoke invite | `DELETE …/pending-invitations` | Bearer | JWT |
| Remove member | `DELETE /organizations/{org_id}/users/{user_id}` | Bearer | JWT |

### 7.1 Header policy (normative)

- Bearer routes: `X-Tenant-ID` **optional**; never authoritative.
- Public API edge **may** strip `X-Tenant-ID` and `Cookie` (current `sesame-idam-api-edge`).
- OpenAPI must not mark `X-Tenant-ID` `required: true` on bearer routes that the edge strips.

### 7.2 Error shapes

Machine-readable `error` codes must match published enums so response validation does not turn a missing-parameter message into an opaque 500. Prefer `unauthorized` / `validation_error` / `forbidden` consistently on consumer-facing routes.

---

## 8. Data model (Sesame)

| Table | Role |
|-------|------|
| `sesame_idam.users` | Identity; RLS by `tenant_id` |
| `sesame_idam.organizations` | Workspace; `tenant_id` |
| `sesame_idam.org_memberships` | Active (and status) memberships |
| `sesame_idam.org_invites` | Pending invites: email, role, token, expiry, accepted_at |

### 8.1 RLS requirement for member listing

Any read of `users.email` (or profile fields) in the list-members path **must** run with transaction/session tenant context set to the JWT `tenant_id` (or an equivalent tenant-scoped join that satisfies RLS). Silent fallback to `user-{uuid}` when the user exists is a **defect**, not acceptable UX.

---

## 9. Invite email

| Requirement | Detail |
|-------------|--------|
| Transport | SMTP (dev: Mailpit / cluster `mailpit` in `data`) |
| Content | Org display name, role, magic-link URL, expiry |
| Link shape | Product-configured base + token query (e.g. `https://loadlinker…/onboarding?token=…`) |
| API | Always return `invite_token` for E2E even when email send succeeds |
| Failure | Persist invite; surface email-send failure in logs + optional warning field; do not pretend email succeeded if SMTP failed |

SMS invite is **out of scope** for v1.

---

## 10. Security

1. Edge strip of `X-Tenant-ID` remains correct; handlers must not require the stripped header.
2. Header/claim mismatch → reject (401).
3. Invite accept: JWT email must match invite email (case-insensitive).
4. Invite tokens: high entropy, single-use on accept, expire (default 7 days).
5. Org-admin authorization on invite/remove/revoke.
6. No cross-tenant membership or invite visibility.

Related finding: [FINDING-2026-07-25-org-mgmt-tenant-header-override](./FINDING-2026-07-25-org-mgmt-tenant-header-override.md) (task 48) — claim-only / reject-on-disagree is mandatory for org-mgmt bearer routes.

---

## 11. Demo seeds (tenant `hauliage`)

Platform fixtures (non-exhaustive):

| Email | Role / org |
|-------|------------|
| `shipper@amecorp.dev` | Owner — AME Corp |
| `transport@transportservices.dev` | Owner — Transport Services |
| `owner@hauliage.dev`, `dispatcher@…`, `driver@…` | Memberships on Transport Services / demos |

Password for local dogfood: documented in consumer integration wiki (not repeated here as a secret). Seeds must remain apply-ordered with platform tenants + RLS grants.

---

## 12. Phased delivery (platform)

| Phase | Scope | Exit criteria |
|-------|--------|----------------|
| **P0** | JWT-bound tenant on `/identity/me`, invite, list-users (OpenAPI + handlers); deploy session + org-mgmt | Public `api.` calls succeed without `X-Tenant-ID`; invite not 502 |
| **P1** | RLS-safe member email resolution | List members returns real emails for seeded users |
| **P2** | SMTP invite email + Mailpit verification | Invite creates Mailpit message; token still in JSON |
| **P3** | Accept-invite + active-org token re-issue hardening | Contract tests / BDD green on Kind |
| **P4** | Align tenant-consumer OpenAPI + conformance fixtures | Consumer contract sync CI green |

Hauliage phases mirror these in the paired PRD.

---

## 13. Acceptance criteria (platform)

- [ ] Bearer `GET /identity/me` without `X-Tenant-ID` returns 200 for a valid token (public edge).
- [ ] Bearer `POST /organizations/{org_id}/invitations` without `X-Tenant-ID` creates invite and returns `invite_token`.
- [ ] Bearer `GET /organizations/{org_id}/users` returns `transport@transportservices.dev` (not `user-a1000001-…`) for Transport Services seeds.
- [ ] Invite email appears in Mailpit when SMTP is configured.
- [ ] Accept invite binds membership; subsequent login/access token includes matching `org_id`.
- [ ] Header `X-Tenant-ID` disagreeing with JWT `tenant_id` → 401 on bearer routes in scope.
- [ ] No regression: login/register still resolve tenant for pre-auth flows.

---

## 14. Open questions (iteration)

1. Exact magic-link URL template ownership: Sesame env vs per-RP redirect allow-list.
2. Whether pending invites appear on `GET …/users` or a dedicated pending collection (affects BFF merge).
3. Role vocabulary: Sesame lowercase slugs vs product uppercase enums — keep mapping in BFF or normalize in Sesame.
4. Timing for webhook-based domain provisioning (ADR-002 S3) vs sync BFF forever for Loadlinker.

---

## 15. Document control

- **Do not** extend superseded account-first checkpoints; update this PRD and the paired Hauliage PRD.
- ADR-002 remains the boundary ADR; implementation backlog for S1/S2 gaps lives here until closed.
- After implementation, update entity wiki pages (`entity-org-invite`, `entity-org-membership`) and append `docs/llmwiki/log.md`.
