# Design: sesame's BFF, sesame's API edge, and moving hauliage onto it

> **Status:** PROPOSED (2026-07-25)
> **Builds on:** DESIGN-north-south-saas-surface.md
> **Tasks:** 50, 51, 54, 55, 56

---

## 1. Agreeing the destination, splitting the component

The destination is right: **all consumption goes north–south through a front
door, and hauliage stops calling cluster DNS.** The rest of this document
assumes that and works out what has to be built.

One correction to the shape. "Sesame needs a BFF the same as hauliage" is true
of *one* of the two things sesame needs, and taking it literally for both would
put a machine-to-machine consumer through a component designed for a browser.

A Backend-For-Frontend exists to serve **one specific frontend**. Hauliage's BFF
aggregates hauliage's backends plus sesame for the loadlinker web app. It is
session-shaped: it holds tokens on behalf of a browser, and its API is whatever
that one UI needs.

Sesame needs two different things:

| | Serves | Shape |
| --- | --- | --- |
| **Sesame BFF** (task 54) | sesame's own platform + tenant consoles | session-shaped: cookie custody, CSRF, screen-driven endpoints |
| **Sesame API edge** (task 50) | third-party relying parties, SDKs, **and hauliage's BFF** | contract-shaped: stable, versioned, credential-authenticated, no session |

Hauliage's BFF is not a frontend. It is a server-side client. Routing it
through a session-oriented BFF would mean giving it cookies it has no use for,
and coupling hauliage's release cycle to sesame's console.

**So hauliage moves onto the API edge — the same surface an external customer
uses.** That is the stronger version of the proposal, for the reason in §2.

---

## 2. Why moving hauliage is worth doing for its own sake

The north–south path has **no real consumer today**. Every control built for it
— A1 rate limits, A4's HSTS and redirect, XFF trust — has only ever been
exercised by tests. The one real consumer sits on the unprotected path. That is
why the controls could be wrong for eighteen months without anyone noticing, and
it is the same root cause as every finding in the stock-take.

Moving hauliage onto the public API inverts that: **the SaaS path stops being
able to rot, because breaking it breaks hauliage.** Rate limits get sized
against real traffic. TLS, the issuer, discovery and JWKS get exercised by a
daily consumer instead of a smoke test. Hauliage becomes tenant zero rather than
a special case.

It also converts an argument into an observation. Today "can an external
customer use this?" is answered by reasoning. Afterwards it is answered by
whether hauliage is up.

---

## 3. What must be true before hauliage is moved

### 3.1 Hauliage needs a credential (task 51)

Today hauliage asserts `X-Tenant-ID: hauliage` and nothing backs it. On the
public edge that is not merely weak, it is unusable: the edge cannot let a
caller name its own tenant in a header, or every customer can name every other.

Hauliage's BFF becomes the **first holder of per-tenant client credentials**,
which is the right forcing function — the credential design gets validated by a
real consumer before any customer depends on it.

### 3.2 The client's hostname derivation will break — silently (task 56)

This is the specific landmine in the path.

`sesame-idam-client` derives the org-mgmt and session base URLs by
**string-replacing `identity-login-service` in `loginUrl`** (`org.rs:145-164`,
`identity.rs:75-88`), with a dev-port variant (`:8101`→`:8104`/`:8102`).

The moment `loginUrl` becomes a public hostname, that substring is gone, the
replacement is a no-op, and **org-mgmt and session calls quietly go to the login
host instead of failing.** Team management and `/identity/me` break in a way
that looks like a sesame bug rather than a config one.

This must be fixed *before* the migration, not during it: explicit per-service
base URLs, and a hard error when one is missing rather than a derivation.

Related, from the same audit: the BFF's hardcoded fallback is
`http://127.0.0.1:8101/...` (`sesame_idam.rs:47`), so a missing config points at
localhost rather than failing closed. Same fix.

### 3.3 The boundary must be real first (tasks 48, 49, 30)

Unchanged from the previous note, and now with a date attached: hauliage's
traffic arriving over the public edge is the first traffic where a header-vs-
token disagreement has an attacker on the other end.

### 3.4 Rate limits must be sized for a real tenant

A1 budgets were set with no production consumer behind them. Hauliage's login
and JWKS volume will be the first real load they see. **Size them against
measured hauliage traffic before cutting over, or the migration presents as an
outage.** This is the most likely way for this change to go wrong on the day.

---

## 4. Sequence

1. **48, 49, 30** — boundary derived from credentials, and tested.
2. **56** — explicit base URLs in `sesame-idam-client`; no derivation, fail
   closed on missing config. *Independent, do it now — it is a latent bug even
   without the migration.*
3. **50** — API edge: own hostname, own route, own budgets, split from console
   nginx.
4. **51** — per-tenant client credentials; hauliage's BFF is the first holder.
5. **54** — sesame's own BFF for the platform and tenant consoles (unblocks
   task 31's screens).
6. **52** — per-tenant quota, sized against measured hauliage traffic.
7. **55** — cut hauliage over. Cluster DNS to sesame is then removed, not left
   as a fallback.

Step 7's last clause matters. If the east–west path stays reachable "just in
case", the migration has added a hop and kept every problem it was meant to
close — and the unprotected path is the one that gets used the next time
something is urgent.

---

## 5. What this does not decide

> **Open:** whether sesame's consoles get one BFF or two. The platform console
> and tenant console have deliberately disjoint authority (ADR-011 §1), and one
> BFF holding both sessions is a component that can confuse them. Two is more
> plumbing and one fewer way to breach the boundary that the entire product
> sells.

> **Open:** whether hauliage's BFF keeps forwarding end-user tokens once it
> holds client credentials, or moves to RFC 8693 delegation (task 39). The
> migration does not force this, but it is the natural moment.
