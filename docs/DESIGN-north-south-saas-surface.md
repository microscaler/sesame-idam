# Design: the north–south surface is the product

> **Status:** PROPOSED (2026-07-25)
> **Supersedes the framing in** STOCKTAKE-2026-07-25 §10, which ordered work by
> internal exposure and treated external consumption as later.
> **Related:** ADR-011 (authority separation), ADR-012 (service identity),
> ADR-010 (frontend architecture).

---

## 1. The correction

The previous note asked whether east–west consumption should remain a supported
mode. That was the wrong question, because it treated the two paths as
alternatives of equal standing.

**East–west only works when the consumer is co-located with the identity
provider.** That describes hauliage — one product, one cluster, one operator.
It describes no external customer, ever. An Identity SaaS is *defined* by
serving relying parties the operator does not run.

So the paths are not alternatives:

| | Path | Who it serves |
| --- | --- | --- |
| **North–south** | internet → front door → BFF → sesame | **every customer. The product.** |
| **East–west** | pod → cluster DNS → sesame | hauliage, and only because it happens to share a cluster |

East–west is the special case. It is the deployment accident of the first
tenant, and it has been standing in for the general case because it is the only
one that exists.

**Nothing external can consume sesame today.** The only HTTPRoute in the repo
serves `/` to `sesame-idam-frontend`. There is no API route. What external API
reachability exists arrives *through the console's nginx*, which is a
static-asset server for a UI, currently doubling as the API gateway. That is
the split already flagged as worth doing; it is now load-bearing.

---

## 2. Why this changes the ordering, not just the backlog

Several open items were sized as internal hygiene. Exposure re-prices them,
because the attacker changes from "a workload in our cluster" to "anyone with a
free-tier signup".

**Blocking. Must be closed before any external API route exists:**

| Task | Internal reading | Reading once exposed |
| --- | --- | --- |
| 48 | header/claim precedence bug in one service | **multi-tenancy is a request header.** Any customer names any other customer's tenant |
| 49 | curiosity about proxy behaviour | decides whether 48 is already reachable |
| 30 | boundary tests we owe ourselves | the boundary *is* the product's security claim |
| 34 / 35 | spec hygiene | a public endpoint that may accept `test123` |
| 44 / 47 | admin endpoints need auth | admin endpoints must not be on the public surface at all |

Task 48 is the one to be blunt about: **tenant isolation is currently asserted
by a header the caller sets.** Publishing an API route in that state does not
degrade the security model, it inverts it — isolation would become opt-in for
the well-behaved.

**Promoted from "decide" to "required":**

- **Task 36 — JWKS.** For an Identity SaaS this stops being a question. A
  relying party the operator does not run *must* fetch JWKS to validate tokens.
  Same for `/.well-known/openid-configuration`: it is how every OIDC client
  bootstraps. Both are public by specification, and their rate limits then
  matter for real rather than being untested policy.
- **Task 40 — workload identity.** ADR-012 §2.5 names "production outside one
  trusted cluster" as a trigger. Selling to external relying parties *is* that
  condition, on the roadmap rather than hypothetically.

---

## 3. What does not exist yet at all

These are gaps, not bugs — nothing is wrong, there is simply nothing there.

### 3.1 An API front door distinct from the console

The console's nginx must stop being the API gateway. The API needs its own
HTTPRoute, hostname, rate-limit budgets and TLS posture, so that a UI deploy
cannot change API availability and an API burst cannot starve the console.

→ **Task 50**

### 3.2 A credential an external tenant backend can hold

ADR-011 §1 says a tenant can never hold a platform key. Today that leaves a
tenant's *server-side* application holding nothing at all: every authenticated
call sesame receives carries an end-user token forwarded by a BFF. There is no
answer to "how does a customer's backend authenticate as itself".

That answer is normally per-tenant OAuth client credentials, issued and
rotatable by the tenant admin in their own console, scoped to their tenant by
issuance rather than by header. It is also the natural place for the tenant
boundary to become structural instead of remembered — a credential that *can
only* name its own tenant makes task 48's bug unrepresentable.

→ **Task 51**

### 3.3 Per-tenant quota

Gate A1 budgets are per-route. With one tenant that is the same as per-tenant.
With many it is not: one customer's traffic degrades everyone's login. Per-tenant
limits are also the obvious metering dimension if usage-based pricing is ever
wanted, so the shape is worth getting right once.

→ **Task 52**

### 3.4 A public, stable issuer

`jwtIssuer` is `https://idam.example.com`. `iss` appears inside every token a
customer validates, so it is effectively permanent — changing it invalidates
every relying party's configuration simultaneously. Whether it is one issuer
with a tenant claim or per-tenant issuers is a decision that gets very
expensive to revisit after the first paying customer.

→ **Task 53**

---

## 4. Sequence

1. **48, 49, 30** — make the tenant boundary real and tested. Nothing external
   is safe to route until the boundary is derived from credentials.
2. **53** — decide the issuer shape while it is still free.
3. **50, 36** — API front door, with discovery and JWKS deliberately public.
4. **51** — tenant credentials, which is also when 48 becomes structural.
5. **52** — per-tenant quota, before the second tenant rather than after.
6. **34, 35, 47** — fail-closed and admin-surface hygiene, alongside.

---

## 5. The thing to keep hold of

Every control built so far — rate limits, HSTS, the redirect, XFF trust — sits
on the north–south path, and the only consumer sits on the east–west one. So
the controls have never protected a real consumer, and the real consumer has
never been protected by a control.

Exposing the API resolves that in one direction. It is worth being deliberate
that it resolves it by *moving the consumer onto the protected path*, and not
by quietly leaving a second, unprotected path open behind it.
