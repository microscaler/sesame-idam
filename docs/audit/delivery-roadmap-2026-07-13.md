# Delivery Roadmap — 2026-07-13

> **SCOPE DECISION (2026-07-13):** Deliver the **narrow D3/D4 Hauliage-consumer target**
> first — **product launch in ~6 weeks (target ≈ 2026-08-24)**. Sesame functionality is
> the critical path / major hurdle. The full aspirational surface (RLS bridge, TS SDK,
> hosted UI, 100% PropelAuth parity, hybrid online-fallback authz) is **explicitly
> deferred to a post-launch track** — see Appendix A.
>
> **Inherits from:** [`epic-delivery-audit-2026-07-10.md`](./epic-delivery-audit-2026-07-10.md)
> (delivery tiers D0–D6, stub-vs-impl matrix, Waves A–D),
> [`first-delivery-wave-a.md`](./first-delivery-wave-a.md) (D1–D3 staging),
> [`../ADR-002-tenant-consumer-idam-api-boundary.md`](../ADR-002-tenant-consumer-idam-api-boundary.md)
> (tenant-consumer boundary, phases S0–S3).
>
> **Gate (non-negotiable, per CONTRIBUTING):** every endpoint = compile + clippy-pedantic
> + unit + **BDD E2E** + `// BRRTRouter: user-owned` sentinel + truthful
> `x-brrtrouter-impl` marker before it counts as done.

---

## Launch scope = D3 + D4 (Hauliage consumer subset only)

**D3 — MVP identity surface:** email/password login + register, refresh (401-correct),
logout, `/identity/me`, api-keys validate. **Mostly real today.**

**D4 — B2B org platform (consumer subset per `openapi/idam/tenant-consumer/openapi.yaml`):**
the exact Hauliage-facing surface is just seven consumer paths plus core auth:

| Tenant-consumer path | Controller | State (2026-07-13) |
|----------------------|-----------|--------------------|
| `POST /auth/register` | `auth_register` | ✅ real |
| `POST /sessions/active-organization` | `set_active_organization` | ✅ real (Wave A2, typed) |
| `GET /users/me/memberships` | `list_my_memberships` | ✅ real |
| `POST /organizations` | `create_organization` | ✅ real |
| `POST /organizations/{org_id}/invitations` | `invite_user_to_org` | ✅ real |
| `POST /invitations/accept` | `accept_invitation` | ✅ real |
| `POST /invitations/preview` | **— none —** | ❌ **controller missing** |

**D4 org-admin surface Hauliage will exercise (currently stub):**

| Operation | Controller | State |
|-----------|-----------|-------|
| `GET /organizations/{id}` | `fetch_org` | 🔴 stub |
| List org members | `fetch_users_in_org` | 🔴 stub |
| Change member role | `change_user_role_in_org` | 🔴 stub |
| Remove member | `remove_user_from_org` | 🔴 stub |
| Revoke pending invite | `revoke_pending_invite` | 🔴 stub |
| Role→permission mapping | `principal_effective` returns `permissions: []` | 🔴 not wired |

**That is the entire sesame gap to launch.** Not 119 endpoints — roughly **7 real
controllers + 1 authz wiring + revocation minimum + E2E**. The long tail (SCIM, SSO,
webhooks-admin, OTP/social login, MFA, user-mgmt admin CRUD) is **out of launch scope**.

---

## Where the last agent stopped (2026-07-12)

Wave A (D1–D3) is functionally done but **uncommitted on the ms02 working tree**. Live
BDD green: `account_first` 2/2, `token_lifecycle` 2/2, `users_me` 6/6, `api_keys` 14/14.
Residue: **A6** (Hauliage Playwright E2E, needs live stack, hauliage side) and **A8**
(k8s `database-env.yaml` + `:8080` parity, helm ready, verify on Kind).

---

## 6-Week Plan

> Each week ends with the gate above. Run BDD on ms02
> (`KUBECONFIG=../shared-k8s-cluster/kubeconfig/shared-k8s.yaml`, postgres/redis in ns `data`).

### Week 1 — Land the base + freeze the contract *(closes D2)*
- Commit + push uncommitted Wave A (sentinels intact).
- Verify **A8** (`database-env.yaml`, `:8080` ClusterIP) on live Kind.
- Hand **A6** to hauliage (`REAL_LOGIN=1` Playwright).
- **Freeze the launch endpoint list:** diff `tenant-consumer/openapi.yaml` against the
  Hauliage BFF client's actual calls → lock exactly which org-admin paths are in scope.
  (Reconcile counts for *this subset only* — skip the full F-018 119-endpoint audit.)

### Week 2 — Org read surface
- `fetch_org` (`GET /organizations/{id}`) — real impl + BDD + tenant-isolation test.
- `fetch_users_in_org` (member list) — real impl + BDD + tenant-isolation test.
- `invitations/preview` — **new controller** (spec exists, impl absent) + BDD.

### Week 3 — Invite + membership lifecycle
- `revoke_pending_invite`, `remove_user_from_org`, `change_user_role_in_org` — real impl + BDD each.
- Completes the D4 consumer org-admin subset.

### Week 4 — Roles/permissions for Hauliage personas
- Wire **role→permission mapping** in `principal_effective` (C3) so shipper/transporter
  JWTs carry real `permissions`, not just role names. This is the D4 "roles" promise and
  what Hauliage authz keys off.
- Verify **A7 role-split seeds** (`shipper@amecorp.dev`, `transport@transportservices.dev`)
  produce correct JWT claims end-to-end.

### Week 5 — Revocation minimum + security-for-launch
- Wire **jti denylist** (B2) + **consumer `ver` rejection** (B4) — minimum viable
  revocation so logout / member-removal actually invalidates tokens. **Security-critical;
  cannot launch without it.**
- **Decision gate:** api-keys rotation (B5) — in only if Hauliage workers need it.
- **Decision gate:** ADR-002 **S3 webhooks / async worker provisioning** — in or out?
  Default OUT for launch (use synchronous provisioning or seeds); flag if Hauliage
  company-profile provisioning depends on it.

### Week 6 — E2E + launch readiness
- Full **account-first → create org → invite → accept → role → BFF** Playwright E2E green
  on the live stack (closes A6).
- **Tenant-isolation regression suite** across every new controller (doubles as the Epic 8
  isolation test — the one hardening item that *is* in launch scope).
- Truthful `x-brrtrouter-impl` markers; refresh `openapi_example_coverage.csv`; update
  `Epics/INDEX.md`.
- Buffer / bugfix / redeploy verification.

---

## Explicitly OUT for launch (post-launch track — Appendix A)

RLS bridge SQL · TypeScript SDK · hosted UI · full 119-endpoint surface · hybrid
online-fallback authz (C1/C2) · SCIM/SSO admin · webhook delivery system · OTP / social /
magic-link login variants · MFA enrollment · user-mgmt admin CRUD · delegation/`act` ·
DPoP · caching layer · ES256/HSM.

> **RLS bridge note:** it's the README headline, but it secures the *consuming app's* DB.
> Hauliage enforces its own tenancy, so the bridge is almost certainly **not** on the
> launch critical path — confirm with hauliage, default OUT.

---

## Risks to the 6-week date

| Risk | Mitigation |
|------|------------|
| Hidden Hauliage-required endpoint outside the frozen list | Week 1 contract freeze against the *actual* BFF client, not the spec alone |
| Revocation (Week 5) slips → insecure logout at launch | Treat B2+B4 as a launch blocker, not a nice-to-have; do not cut |
| S3 webhook provisioning turns out to be required | Decide in Week 5, not Week 6; keep a synchronous fallback ready |
| Cross-repo A6 blocked on hauliage stack availability | Hand off Week 1, not Week 6; sesame BDD proves the chain independently |
| Stub mistaken for done | Sentinel + truthful `x-brrtrouter-impl` enforced at each week's gate |

---

## Appendix A — Post-launch aspirational surface (deferred)

Retained from the original 2026-07-13 roadmap for when launch is secured. Delivers the
full README vision beyond D4.

- **Phase 2 — RLS bridge:** `sesame_set_session()` / `sesame_current_*()` SQL helpers +
  `SesameExecutor` (Lifeguard wrapper) + zero-bleed BDD. Headline differentiator; 0% today.
- **Phase 3 — Hybrid authz (Wave C):** route classification (C1, needs full endpoint
  reconciliation), selective per-request online fallback (C2), entitlements
  ref/hash/cache wiring (C4/C5), refresh-reuse detection (C6), `typ=at+jwt`/algorithm
  hardening (C7), Prometheus JWT metrics + key-age alerting (C8).
- **Phase 4 — Full API surface (Wave D):** per-service stub→impl→BDD conveyor —
  user-mgmt admin, org-mgmt admin (SSO/SCIM/webhook delivery with retries+HMAC),
  api-keys full lifecycle, login variants.
- **Phase 5 — Developer contract:** TypeScript SDK (`@sesame-idam/frontend` + admin
  client), hosted login/onboarding UI, integration guide.
- **Phase 6 — Defer:** DPoP / RFC 8693 exchange, ES256 co-default, HSM, delegation/`act`
  (Epic 6), full Epic 7 caching.

---

*Authored 2026-07-13. Re-scoped same day to the 6-week D3/D4 Hauliage launch target.
Extends the Waves A–D model in `epic-delivery-audit-2026-07-10.md` §12.*
