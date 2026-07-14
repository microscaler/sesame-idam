# Sesame-IDAM — Launch 1.0 Roadmap

> **Goal:** ship a Sesame-IDAM that platform engineers **envy** — and that makes
> BRRTRouter the obvious choice — by delivering on the README's promise:
> *auth with zero logic in your app, standards-based asymmetric JWTs, and
> database-native RLS security nobody else offers.*
>
> **Authored:** 2026-07-13. **Expanded:** 2026-07-13 in the
> [Launch 1.0 specification set](./roadmap/launch-1.0/README.md). This strategic roadmap
> governs the general-availability product scope. The narrower
> [Hauliage delivery roadmap](./audit/delivery-roadmap-2026-07-13.md) governs the preceding
> **test-user enablement milestone**: just enough IDAM to onboard initial Hauliage users.
> Its testable scope is defined in the
> [Hauliage enablement specification](./roadmap/launch-1.0/hauliage-test-user-enablement/README.md).
>
> **Grounded in:** the competitive gap analysis (Sesame ≈12% of a full IdP today;
> ~16/136 endpoints real) and the delivery-tier model (D0–D6) in
> [`audit/epic-delivery-audit-2026-07-10.md`](./audit/epic-delivery-audit-2026-07-10.md).

---

## 1. Positioning — the wedge

Every competitor forces a trade-off Sesame can refuse:

| Competitor | Their gap Sesame attacks |
|---|---|
| **PropelAuth** | Proprietary, opaque session tokens, no DB-level security |
| **Clerk** | Closed SaaS, per-MAU pricing, B2C-first, no self-host |
| **Auth0/Okta** | Expensive, closed, no RLS, integration-heavy |
| **WorkOS** | Enterprise-add-on only, not a full IdP, closed |
| **Supabase Auth** | RLS yes, but flat users — no real B2B org model |
| **Keycloak/Ory** | OSS but operationally brutal, no RLS bridge, dated DX |

**Sesame's unique triple** (no competitor has all three): **open-source + self-hosted**
· **standards-based asymmetric JWT/JWKS** · **database-native RLS bridge**. The launch
must make that triple *real and demoable*, then wrap it in DX good enough to envy.

**BRRTRouter tie-in:** Sesame is the flagship reference app for BRRTRouter (OpenAPI→code,
`may` coroutine runtime, Lifeguard ORM). "Adopt BRRTRouter and you get Sesame-grade
security + RLS for free" is the ecosystem pitch. Every launch deliverable should also be a
BRRTRouter proof point.

---

## 2. What's already real (credit where due)

- ✅ **Asymmetric EdDSA JWT + JWKS** with rotation/grace/revoke — production-grade; *ahead*
  of PropelAuth/Clerk's opaque tokens. This is the launch's credibility anchor.
- ✅ Refresh rotation + reuse detection + family revocation.
- ✅ Password login/register/logout, `/identity/me`, OIDC discovery.
- ✅ B2B org lifecycle (create/fetch/invite/preview/accept/memberships) with **tenant
  isolation proven at the data layer**.
- ✅ Login→authz-core enrichment path; per-service JWKS validation.
- 🟡 API-key validate; `principal/effective` (roles, no permissions); revocation *write-path*.

**Everything below is what stands between this and a product.** Each item ships only when it
clears the repo gate: `cargo check` + `just lint-rust` (clippy pedantic) + unit + **BDD E2E**.

---

## 3. The launch thesis — sequencing

Lead with the **moat** (security + RLS), because that's the envy. But interleave
**table-stakes** so the product never looks toy. Order:

```
P0 Harden the core        → close the one security gap (revocation enforcement)
P1 THE RLS BRIDGE         → the killer feature; make it real + demoable   ← headline
P2 Complete the auth surface (table-stakes: user-mgmt, MFA, social, passwordless)
P3 B2B/RBAC + enterprise wedge (permissions-in-token, API keys, webhooks, SSO/SCIM)
P4 THE DEVELOPER CONTRACT  → SDK + hosted UI + "auth in an afternoon"     ← envy engine
P5 Trust & scale          → audit UI, breach/bot defense, DPoP, SOC2 readiness
```

**Launch 1.0 GA = P0 + P1 + P2 + P4** (moat + credible auth surface + the DX that sells it).
The bounded D3/D4 Hauliage cut is a **test-user enablement milestone**, not a Sesame product
release and not a competing definition of GA.
**P3 enterprise depth + P5 trust** land as **1.1/1.2** fast-follows. See the
[scope evaluation](./roadmap/launch-1.0/README.md#roadmap-evaluation) and rationale below (§6).

---

## 4. Phased roadmap

### P0 — Harden the core *(1–2 wks)*

[Detailed requirements, acceptance criteria, and exit gate](./roadmap/launch-1.0/p0-harden-core/README.md)

Close the credibility gaps in the part that already works.
- **Revocation enforcement (read-side).** Write-path exists; add denylist check to the
  validation path. **Decision:** do it in **BRRTRouter's `JwksBearerProvider`** (one place,
  every consumer benefits, fail-closed with a bounded Redis deadline) — this is a BRRTRouter
  feature that *sells* BRRTRouter.
- `typ=at+jwt` + algorithm hardening (RFC 9068), consumer `ver` rejection on version bump.
- **Gate:** revoked access token is rejected end-to-end; BDD proves it.

### P1 — The RLS Bridge *(3–4 wks) — THE HEADLINE*

[Detailed requirements, acceptance criteria, and exit gate](./roadmap/launch-1.0/p1-rls-bridge/README.md)

The README's killer feature. The first production-shaped slice is now delivered; complete the GA
contract and make it the demo everyone screenshots.
- Ship `rls_set_session()` / `sesame_current_organization_id()` / `sesame_current_*()` SQL as
  a **versioned, deploy-once artifact** (+ migration + docs).
- **First-class Lifeguard contextual transactions** on the base executor/pool types: inject the
  versioned Sesame context transaction-locally from validated JWT claims. No parallel
  `SesameExecutor` type and no session-scoped GUC state.
- **Zero-bleed proof suite**: property tests + BDD showing RLS policies filter rows by
  `org_id`/`user_type` with a failsafe (NULLIF → zero rows), across tenants.
- **The "wow" demo**: a sample app where `SELECT * FROM invoices` returns only the caller's
  org's rows with *no WHERE clause in app code* — because Sesame+RLS did it.
- **Gate:** a consuming app enforces tenant isolation at the DB with zero auth logic; demo +
  guide published.

### P2 — Complete the auth surface *(4–6 wks) — table-stakes*

[Detailed requirements, acceptance criteria, and exit gate](./roadmap/launch-1.0/p2-auth-surface/README.md)

So it never looks like a toy next to Clerk. `identity-user-mgmt` is **0/26 real** today.
- **User management**: get/update/disable/enable/delete/search (+ tenant-isolation BDD each).
- **MFA**: TOTP enrollment/verify + recovery codes; step-up (files exist).
- **Verification flows**: email + phone (OTP) verification.
- **Social OAuth**: Google, Microsoft, GitHub (the 80% providers) via the `may_http` client.
- **Passwordless**: magic links + email/SMS OTP login.
- **Gate:** a user can register→verify→enable MFA→social-login; all BDD-covered.

### P3 — B2B/RBAC + enterprise wedge *(5–7 wks) — the PropelAuth/WorkOS fight*

[Detailed requirements, acceptance criteria, and exit gate](./roadmap/launch-1.0/p3-b2b-enterprise/README.md)

- **Permissions in the token**: wire org-mgmt role→permission tables into `principal/effective`
  (returns `permissions:[]` today) so JWTs carry fine-grained perms; add `POST /authorize`
  (RFC 7662-style) for the hybrid path.
- **API keys**: full lifecycle — create/rotate/scope/rate-limit/archive (personal + org).
- **Webhooks**: real delivery — HMAC signing, retries, delivery tracking table.
- **Org SSO**: SAML + OIDC per organization (self-service setup, the enterprise checkbox).
- **SCIM**: directory-sync provisioning (the other enterprise checkbox).
- **Gate:** an org admin configures SSO + SCIM self-serve; API keys rotate; webhooks deliver.

### P4 — The Developer Contract *(4–5 wks) — THE ENVY ENGINE*

[Detailed requirements, acceptance criteria, and exit gate](./roadmap/launch-1.0/p4-developer-contract/README.md)

This is what makes engineers *switch*. Without it, Sesame is a raw API; Clerk ships
pixel-perfect components.
- **TypeScript SDK**: `@sesame-idam/frontend` (`useAuth`, `<SignIn/>`, `<OrgSwitcher/>`) +
  backend admin client (`sesame.users.*`, `sesame.orgs.*`) — the exact API the README shows.
- **Hosted login/onboarding UI**: themeable, drop-in — the account-first flow already built.
- **BRRTRouter-native middleware** plus Lifeguard's base-executor RLS capability as first-class
  ecosystem integrations, without adding another executor abstraction.
- **"Auth in an afternoon" quickstart**: one guide, one sample repo, RLS SQL deploy, done.
- **Gate:** a new SaaS integrates login + orgs + RLS in <1 day following the guide.

### P5 — Trust & scale *(ongoing, 1.2+)*

[Detailed requirements, acceptance criteria, and exit gate](./roadmap/launch-1.0/p5-trust-scale/README.md)

- Audit-log **UI + streaming**; admin dashboard.
- **Breach/bot defense**: HIBP breached-password check, brute-force/anomaly throttling.
- **DPoP** (RFC 9449) token binding; delegation/`act` claims; impersonation (stubs exist).
- Device/session listing & revocation.
- **SOC2 Type II** readiness (controls, evidence), data-residency options.

---

## 5. Cross-cutting workstreams (run continuously)

- **Observability (Epic 9):** Prometheus JWT/authz metrics, structured logs, key-age alerts —
  ship *with each phase*, not after.
- **Security regression:** every new controller lands with a tenant-isolation + token-tamper
  test. No exceptions (this is the brand).
- **Truthful surface:** `x-brrtrouter-impl` markers + `openapi_example_coverage.csv` kept
  honest so "stub" is never mistaken for "shipped."
- **Docs-as-product:** the RLS guide, SDK docs, and quickstart are launch deliverables, not
  afterthoughts.

---

## 6. Launch 1.0 definition (the compelling cut)

**1.0 GA ships = P0 + P1 + P2 + P4.** That is: hardened asymmetric-JWT core, the **RLS bridge
(the moat)**, a **complete-enough auth surface** (password/social/MFA/passwordless + user
mgmt), the **B2B org model already built**, and the **SDK + hosted UI + quickstart** that make
it feel like Clerk but self-hosted with RLS.

**Deliberately deferred to 1.1/1.2** (present as roadmap, not blockers): org SSO/SAML, SCIM
(P3 enterprise depth), audit UI, breach/bot defense, DPoP, SOC2 (P5). Reason: the *envy* comes
from RLS + DX, not from SCIM. Ship the wedge; fast-follow the enterprise checkboxes.

**Rough calendar:** P0–P2 + P4 ≈ **3–4 months** with focused effort; P3 + P5 as 1.1/1.2 over
the following 2–3 months. Parallelizable: P1 (RLS) and P2 (auth surface) are independent; P4
(SDK/UI) starts once P2 endpoints stabilize.

The preceding **Hauliage test-user enablement slice** may deploy when its own acceptance gate
is met; it does not imply the GA capabilities above are complete. Its relationship to GA and
the shared quality gate are defined in the [expanded roadmap](./roadmap/launch-1.0/README.md).

---

## 7. "Engineers envy it" — success metrics

- **Time-to-first-protected-route:** < 1 afternoon from zero (quickstart + SDK + RLS SQL).
- **Lines of auth code in the consuming app:** ~0 (JWT validated by middleware, rows filtered
  by RLS).
- **Zero-bleed guarantee:** a published, reproducible proof that cross-tenant reads return zero
  rows even if the app query is naive.
- **Token standard:** RS/EdDSA JWT any OIDC-aware stack can validate via JWKS — no vendor SDK
  lock-in (unlike Clerk/PropelAuth session tokens).
- **Self-host in one `helm install`;** no per-MAU bill.

---

## 8. Risks & honest caveats

| Risk | Mitigation |
|---|---|
| README over-promises vs. ~12% built | This roadmap *is* the reconciliation; update README status honestly until phases land |
| RLS bridge (P1) is novel and the whole thesis rests on it | The base-executor slice and zero-bleed proof are delivered; finish the GA compatibility, benchmark, recovery, sample, and independent-review evidence before representing P1 as accepted. |
| Revocation enforcement touches BRRTRouter (shared) | Keep the accepted fail-closed, bounded Redis policy consistent across every consumer; it doubles as a BRRTRouter selling feature. |
| Enterprise SSO/SCIM (P3) is deep | Explicitly a 1.1 fast-follow, not 1.0 — don't let it block the wedge |
| Solo/small team vs. 3–4mo scope | Parallelize P1‖P2; treat SDK/UI (P4) as the highest-leverage adoption spend |

---

## 9. Immediate next actions

1. **P0 completion:** finish cross-consumer live evidence for the delivered fail-closed denylist
   and version checks in `JwksBearerProvider`.
2. **P1 completion:** extend the delivered base-executor policy slice with the GA compatibility,
   benchmark, recovery, sample, and independent-review evidence required by the P1 exit gate.
3. **Reconcile `docs/propelauth-gap-analysis.md`** to reality (flag overstated coverage) and
   trim README status claims to match this roadmap.

---

*The bar is not "matches PropelAuth's feature list." The bar is: an engineer reads the RLS
quickstart, ships tenant-safe auth in an afternoon with zero auth code, and tells their team.
Everything here serves that moment.*
