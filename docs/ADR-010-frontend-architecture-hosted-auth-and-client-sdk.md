# ADR-010: Frontend architecture — hosted auth surface, platform/tenant consoles, client SDK

> **Status:** PROPOSED (2026-07-24)
> **Deciders:** Platform (Sesame-IDAM), Microscaler product teams
> **Related:** [ADR-004](./ADR-004-platform-tenant-provisioning.md) (platform vs
> tenant), [ADR-007](./ADR-007-tenant-domain-verification.md) (verified custom
> domains), [ADR-008](./ADR-008-authentication-assurance-passkey-first-mfa.md)
> (passkey-first, phishing resistance), [ADR-009](./ADR-009-multi-tenant-sms-sender-identity-and-credential-custody.md)
> (per-tenant sender/branding config). Prior art: `PriceWhisperer/ui`
> (SolidJS + Vite + Tailwind + Playwright, `shared/` + `shared-portals/`
> pattern) and the existing bare stubs `sesame-idam/ui/{ax,cx,brochure}-frontend`.

---

## 1. Context

Sesame has reached the point where it needs first-class UIs: a marketing
brochure, a **platform operator console** (manage tenants, environments, and
the platform-admin surfaces from ADR-004/009), and a **tenant admin console**
(manage users, orgs, applications, branding, and per-tenant domain/SMS/OAuth
config). It must also answer the recurring integration question: *how does a
tenant put a Sesame login on its own `/signin`?*

Three constraints are already decided elsewhere and drive the answer:

- The README promise — **auth with zero logic in the tenant's app**.
- **ADR-008** — passkeys/WebAuthn are **origin-bound**; phishing resistance
  only holds when authentication happens on ONE trusted, verified origin.
- **ADR-007** — a tenant can run under its own verified custom domain.

Current state: `ui/{ax,cx,brochure}-frontend` are bare Solid-Vite template
scaffolds (no Tailwind, no shared design system); `PriceWhisperer/ui` carries
the real look-and-feel and the shared-portals pattern to standardise on.

---

## 2. Decisions

### 2.1 Standardise on `./frontend/` with a shared design system

Consolidate all Sesame UI under `sesame-idam/frontend/`. Stack: **SolidJS +
Vite + TypeScript + TailwindCSS + Playwright** (matches PriceWhisperer and the
existing stubs). A `frontend/shared/` design system (tokens, components, auth
button, layout) is extracted from the PriceWhisperer look-and-feel and consumed
by every app.

```
frontend/
  shared/        design system: tokens, components, layout, api client base
  brochure/      marketing site                  (← ui/brochure)
  platform/      Sesame operator console         (← ui/ax-frontend)
  tenant/        tenant admin console            (← ui/cx-frontend)
  auth/          hosted auth surface (Universal Login) — the canned pages   [NEW]
  client-sdk/    @sesame/idam-client — redirect + session library           [NEW]
```

`ax-frontend → platform`, `cx-frontend → tenant`, `brochure` unchanged in
purpose. The two new members are the substance of this ADR.

### 2.2 Canned login = HOSTED redirect surface, not an embedded form

Sesame provides canned login pages as a **hosted auth surface** the tenant
**redirects to** — NOT a credential form the tenant embeds in its own origin.
The tenant's `/signin` becomes a branded button that redirects to the hosted
surface and returns with a session.

Rationale (each point is a prior decision, not a preference):

- **Phishing resistance (ADR-008).** WebAuthn is origin-bound. One trusted
  origin is the only way passkeys deliver. Embedding the form in each tenant
  origin either breaks passkeys or fragments the RP model.
- **Zero logic (README).** Redirect-and-read-session means the tenant app never
  handles a password, OTP entry, or passkey ceremony.
- **Branding without compromise (ADR-007).** The hosted surface runs under the
  tenant's verified custom domain (`login.tenant.com`) with tenant theming
  (logo/colours from the ADR-009 tenant config) — hosted ≠ unbranded.
- **Smaller attack surface.** Credentials live in one origin, patched in one
  place. This is the proven Universal-Login / AuthKit pattern.

### 2.3 `@sesame/idam-client` — the thing tenants actually bake in

A tiny TypeScript SDK is the tenant-facing integration point:
`login()` (redirect to hosted), `handleCallback()`, `getSession()`, silent
refresh, `logout()`, and JWKS-validated session checks. Framework-agnostic
core + optional Solid/React adapters. It bakes in the **redirect + session
handling**, delivering "zero auth logic" — the credential form stays hosted.

### 2.4 Embedded inline widget is deferred and origin-safe only

For tenants who insist on an inline form, the only sanctioned option (later) is
an **iframe pointing at the hosted origin**, so credentials still execute in
Sesame's origin. A native in-tenant-origin form is NOT offered — it cannot
satisfy ADR-008. This is explicitly secondary.

### 2.5 App → audience → auth mapping

| App | Audience | Auth |
|---|---|---|
| brochure | public | none |
| platform | Sesame operators | platform-admin auth (ADR-004 platform routes) |
| tenant | tenant admins/staff | tenant-scoped auth via the hosted surface |
| auth | end-users + admins | IS the auth surface (login-service endpoints) |
| client-sdk | (library) | consumed by tenant apps + the two consoles |

Both consoles authenticate **through** the hosted `auth` surface via the SDK —
they are the first dogfood consumers of the exact integration path external
tenants use.

---

## 3. Consequences

**Positive**

- The canned-login story is the phishing-resistant one, aligned with the
  launch's top pre-launch item (ADR-008 passkeys).
- Tenants integrate with a few lines (SDK redirect), never touching credentials
  — the README promise made literal.
- One design system; consoles dogfood the tenant integration path.
- Per-tenant branding via ADR-007 + ADR-009 config with no security trade-off.

**Negative / follow-up**

- The hosted `auth` app + SDK are net-new build (vs. shipping a form snippet).
- Per-tenant theming needs a theme-resolution path (tenant config → hosted
  surface) — ties to ADR-009's config surface.
- Redirect UX (vs. inline) is a deliberate trade for security; smooth it with
  fast redirects and silent refresh.

---

## 4. Open questions

> **Open:** Monorepo tooling for `./frontend/` — pnpm workspaces + Turborepo,
> or keep independent Vite apps sharing `shared/` by path? Leaning pnpm
> workspaces for `shared/` + `client-sdk/` reuse.

> **DECIDED (2026-07-24):** `client-sdk` is **vendored** — a private
> workspace package (`"private": true`, no npm publish) consumed by the
> consoles and dogfood tenants via the workspace. Publishing to npm happens
> at GA, when the public API surface is stable enough to carry semver
> commitments.

> **Open:** Does the tenant console need its own hosted-surface theme preview,
> or is theming edited in config and previewed live on the auth app?

---

## Sources

- Prior art: `PriceWhisperer/ui` (SolidJS + Vite + Tailwind + Playwright).
- Existing stubs: `sesame-idam/ui/{ax,cx,brochure}-frontend`.
