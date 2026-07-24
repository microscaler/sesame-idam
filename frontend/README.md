# Sesame frontend

Implements [ADR-010](../docs/ADR-010-frontend-architecture-hosted-auth-and-client-sdk.md).
Supersedes the old `ui/{ax,cx,brochure}-frontend` scaffolds.

```
frontend/
  shared/      design system: Tailwind preset, tokens, components, runtime tenant theming
  client-sdk/  @sesame/idam-client — redirect + session handling (VENDORED until GA)
  auth/        hosted auth surface — the canned login pages tenants redirect TO
  platform/    Sesame operator console
  tenant/      tenant admin console
  brochure/    public marketing site
```

Stack: **SolidJS + Vite + TypeScript + TailwindCSS**, pnpm workspace.

## Quickstart

```bash
cd frontend
corepack enable && pnpm install
pnpm dev:auth        # hosted auth surface   (also: dev:platform, dev:tenant, dev:brochure)
pnpm build           # all apps
pnpm typecheck       # all packages
```

Dev proxies `/idam` → `http://localhost:8080` (override with `VITE_IDAM_PROXY`),
so the browser sees one origin and dev needs no CORS exception. Production
relies on the Gate A5 origin allow-list.

## The two ideas that matter

**1. Canned login = hosted surface, not an embedded form.** Tenants redirect
to `auth/`; the credential ceremony (password, OTP, magic link, and later
passkeys) happens on one trusted origin. This is what makes ADR-008 passkeys
work at all (WebAuthn is origin-bound) and delivers "zero auth logic in your
app". Tenants integrate with `@sesame/idam-client`:

```ts
const sesame = createClient({
  authBaseUrl: 'https://login.tenant.com',   // ADR-007 verified domain
  tenantId: 'hauliage',
  redirectUri: 'https://app.tenant.com/callback',
});
sesame.login();                     // /signin button
await sesame.handleCallback();      // /callback
await sesame.getSession();          // anywhere (silent refresh)
```

**2. Mechanics from Sesame, appearance from the tenant.** The login pages are
stylable per tenant *at runtime* — `applyTenantTheme()` sets CSS variables
(brand colour, logo, radius) from the tenant's config, and the app is served
under the tenant's own verified domain. One hosted bundle, many brands; the
flow logic, rate limits, lockout and enumeration-safety stay Sesame's.

## Design language

Consoles follow a stripped-down, **status-first** aesthetic (inspired by the
Flux Operator web UI): light/dark first-class, minimal chrome, colour reserved
for state semantics (`StatusPill`), dense but calm layouts (`ConsoleShell`).
Tokens come from the PriceWhisperer look-and-feel via
`shared/tailwind.preset.js` so the Microscaler family stays coherent.

## UI ↔ backend contract notes

- OTP / magic-link **send** endpoints always return a generic success (Gate A3:
  no enumeration, no cap oracle). The UI therefore advances to "check your
  inbox/phone" unconditionally — **never** branch on whether an account exists.
- Login and OTP-verify failures return one indistinguishable 401
  (`invalid_credentials`) covering wrong secret, unknown user, and lockout
  (Gate A2). Show the server's message; don't infer more.
- Per-login SMS is disabled by default (cost policy, ADR-009): the phone-OTP
  path is present but returns generic success without sending unless the
  environment opts in.

## Status

Scaffold. Apps build and typecheck; the consoles render their shells with
placeholder cards. Not yet wired: `/session/exchange` (the SDK's code→session
endpoint — `auth/` currently dev-round-trips tokens through sessionStorage),
real console data, per-tenant theme fetch, passkeys (ADR-008), Playwright e2e.
