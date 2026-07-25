# ADR-013: one public domain, and where the issuer lives

> **Status:** ACCEPTED (2026-07-25)
> **Supersedes:** the two-domain proposal in DESIGN-north-south-saas-surface.md
> and DESIGN-sesame-bff-and-consumer-migration.md.
> **Closes:** task 53.

---

## Decision

Everything public is served from **`sesameidentity.com`**. `.io` is abandoned
rather than renewed.

| Host | Serves |
| --- | --- |
| `sesameidentity.com`, `www.` | brochure |
| `app.sesameidentity.com` | platform console + tenant console (via the sesame BFF, task 54) |
| `api.sesameidentity.com` | the API edge (task 50) — relying parties, SDKs, hauliage |
| `id.sesameidentity.com` | **issuer**: OIDC discovery and JWKS, nothing else |

`iss = https://id.sesameidentity.com`.

---

## Why a separate issuer host on the same domain

`iss` is the one string that is effectively permanent — it lives in every
relying party's configuration, and changing it is their migration, not ours,
on their schedule.

Giving it a dedicated host keeps two things away from it:

- **The brochure.** If `iss` were the apex, `/.well-known/openid-configuration`
  would live on the marketing site, one CMS route change away from an outage
  across every customer's login.
- **The API host.** Endpoints listed *inside* the discovery document can move
  freely; the issuer identifier does not have to move with them. `api.` can be
  re-platformed, split, or renamed without touching a single customer's config.

`id.` is deliberately boring and describes no product, so there is no rebrand
that wants it back.

---

## What was given up, and why it does not matter

The two-domain proposal's real argument was not branding: separate registrable
domains mean the browser **cannot** attach a console session cookie to an API
request, making CSRF against the API unrepresentable rather than defended.

That argument was right about the goal and wrong about the mechanism. **The API
edge strips `Cookie` on ingress and authenticates solely from `Authorization`.**
That is strictly stronger than the domain split:

- it does not depend on browser cookie scoping, `__Host-` prefixes, or any
  developer remembering the rule
- it holds for non-browser callers too, which is most of the API's traffic
- **it is testable.** A request carrying a session cookie and no bearer must get
  401, and that is a conformance test rather than an argument

The domain split would have delivered this guarantee only for browsers, only
while nobody set a `Domain=` attribute. Doing it at the edge delivers it
unconditionally.

→ **Task 57**

## What is gained

Two costs of the split disappear:

1. **No cross-site cookie exposure.** With console and API on separate
   registrable domains, every console→API request is *cross-site*, and browsers
   are progressively restricting cross-site cookies. Same-site removes a
   standing dependency on browser policy that was going to move under us.
2. **No split-issuer SDK risk.** The earlier plan put `iss` and `jwks_uri` on
   different registrable domains, which is spec-legal but which some OIDC
   libraries reject. That gate is gone; everything is same-site.

## What gets worse

**Subdomain takeover matters more.** On one registrable domain, a dangling DNS
record for a deprovisioned host is a foothold for setting cookies scoped to the
parent, and for phishing that survives inspection of the address bar. Task 57
blunts the cookie half. The rest needs hygiene: no wildcard CNAMEs to third
parties, and monitoring for dangling records.

→ **Task 58**

---

## Consequences

- Task 53 is closed by this ADR.
- Dev must mirror the shape — `app.`, `api.`, `id.` under the local domain — so
  that cookie behaviour and the ingress strip are exercised where people work
  and not first observed in production.
- A wildcard certificate on `*.sesameidentity.com` covers all four, and covers
  per-tenant hosts if those are ever wanted.
- Typo-neighbour domains are worth owning. An identity provider is the
  highest-value phishing target on the estate.

## Note

The `.io` question was a useful forcing function rather than the real issue. It
made us ask where `iss` lives and what depends on it, and that question would
have been worth answering even if the Chagos treaty had never been signed —
rebrands and acquisitions move domains too. The answer, `id.`, is the one we
would want regardless.
