# ADR-011: Platform and tenant are disjoint authority domains

> **Status:** ACCEPTED (2026-07-25)
> **Deciders:** Platform (Sesame-IDAM), Microscaler product teams
> **Related:** [ADR-002](./ADR-002-tenant-consumer-idam-api-boundary.md) (tenant
> consumer API boundary), [ADR-004](./ADR-004-platform-tenant-provisioning.md)
> (platform provisioning), [ADR-005](./ADR-005-first-class-rls-contract.md) (RLS
> contract), [ADR-009](./ADR-009-multi-tenant-sms-sender-identity-and-credential-custody.md)
> (SMS sender identity), [ADR-010](./ADR-010-frontend-architecture-hosted-auth-and-client-sdk.md)
> (frontend architecture).

---

## 1. Context

In hosted SaaS mode a tenant has **no association with the platform**. They are
a different company, billed separately, staffed by different people who have
never met the platform operators. A tenant administrator is not a junior
platform administrator — they are outside the organisation entirely.

That has a consequence the code did not reflect: **a tenant must never be
expected to hold a platform credential, role, or access path.** Not "should
avoid"; must not, because there is no mechanism by which they could safely be
issued one and no relationship in which they could be trusted with it.

Two features shipped in violation of this without anyone noticing:

- **SMS sender configuration** (ADR-009 Phase 2) was designed for tenant
  self-service — "tenants will enter theirs in the UI and be stored to the
  Database" — but the endpoints landed under `/platform/tenants/{slug}/sms/…`
  behind `PlatformServiceAuth` (the `X-Platform-Admin-Key` header). A tenant
  filling in that screen would have to present the platform's operator key.
- **OAuth / SSO provider configuration** has the same shape: a tenant wiring up
  their own Google or Microsoft sign-in goes through
  `/platform/tenants/{slug}/oauth/{provider}`, also platform-key-only.

Both were found by building the tenant console and asking who, concretely, would
be typing into it. Neither was caught by tests, because the tests supplied the
platform key without asking whether the human in the story could ever have it.

The failure is systemic rather than incidental: there was no stated rule to
violate, and the URL prefix `/platform/` was doing the work of an access-control
decision by convention alone.

---

## 2. Decision

### 2.1 Two authority domains, disjoint by construction

| | Platform | Tenant |
| --- | --- | --- |
| Who | Sesame operators, provisioning workers | A customer's own administrators |
| Credential | `X-Platform-Admin-Key` (service key) | End-user JWT with a tenant-admin role |
| URL prefix | `/platform/…` | `/tenant/…` |
| Scope | Any tenant, explicitly named | Exactly one tenant, never named |

**No credential, role, or endpoint crosses.** A platform key is rejected on
`/tenant/*`; a tenant token is rejected on `/platform/*`. Neither is a fallback
for the other, and there is no "escalate" path — a tenant that needs a
platform-only action raises a support request, which is a human process with an
audit trail, not an API affordance.

### 2.2 The tenant identifier comes from the token, never the path

`/tenant/*` endpoints take **no tenant parameter**. The tenant is read from the
verified JWT's claims and nothing else.

This is the same move as ADR-009's `purpose → billing owner` constant map, for
the same reason: an identifier that the caller can supply is an identifier the
caller can tamper with, and every such parameter needs a guard that someone has
to remember to write. Removing the parameter removes the class. There is no
`{slug}` to swap, so cross-tenant access is not "checked and denied" — it is
unrepresentable.

The corollary is that `/tenant/*` handlers must never accept a tenant, org, or
owner identifier in path, query, or body. If one appears, that is the bug.

### 2.3 Tenant-admin is a role, held inside the tenant

A tenant administrator is an ordinary user of that tenant carrying the
`tenant_admin` role. The role is granted within the tenant (by its owner, or at
provisioning time for the first admin) and means nothing outside it. It confers
no visibility of other tenants and no platform capability whatsoever.

`owner` implies `tenant_admin` — the person who owns the tenant can always
administer it, and requiring them to grant themselves a second role would be
a trap rather than a control.

### 2.4 Platform-scoped operations stay platform-scoped

Self-service does not mean everything becomes tenant-reachable. These remain
platform-only, because they are decisions *about* a tenant rather than *by* one:

- tenant creation and deprovisioning (ADR-004 provisioning)
- suspension / status changes — a tenant suspending itself is meaningless, and a
  suspended tenant lifting its own suspension defeats the point
- platform-wide configuration, spend ceilings above a tenant's own budget, and
  the platform's own Twilio credentials (ADR-009 Tier 0)

Where an operator genuinely needs to act on a tenant's own configuration — a
support escalation — the platform-scoped variant stays available and is
audited as an operator action, distinctly from the tenant doing it themselves.

### 2.5 Reading is scoped like writing

A tenant can read only its own configuration, through the same token-scoped
path. There is no "list tenants" for a tenant principal, and no endpoint that
takes a tenant identifier and returns whether it exists — that is an enumeration
oracle, and it leaks the platform's customer list.

---

## 3. Consequences

**Positive**

- The airgap is expressed in the type of the request rather than in reviewer
  vigilance: a `/tenant/*` handler has no tenant parameter to misuse.
- Tenants can self-serve SMS and SSO configuration without a credential nobody
  could have given them.
- Platform operator actions on a tenant remain possible and become
  distinguishable from tenant actions, which is what an audit trail needs.

**Negative / follow-up**

- Two surfaces for the same underlying configuration (tenant self-service and
  operator override), which must not drift. Both delegate to one service layer
  so the behaviour has a single definition.
- A tenant-admin role model and its grant path are new work; until an invite
  flow exists, the first admin is seeded at provisioning.
- Existing `/platform/tenants/{slug}/sms` and `/oauth` consumers keep working;
  the tenant console moves to the tenant-scoped paths.

---

## 4. Testing obligation

The boundary is only real if it is tested from both sides. Every release must
prove:

1. A tenant token cannot reach another tenant's data — there being no parameter,
   this means the resolved tenant always equals the token's tenant.
2. A tenant token is rejected on `/platform/*`.
3. A platform key is rejected on `/tenant/*` — it is not a superset credential.
4. A tenant user *without* the admin role is refused on `/tenant/*`.

Cases 2 and 3 matter most: they are the ones that would silently start passing
if someone "helpfully" made one credential accept the other.

---

## 5. Assurance, and why KYB is built but switched off

Tenant registration needs to answer "is this a real company, and is this
person allowed to act for it". The answers have very different costs, so
assurance is a **ladder** and each risky capability names the rung it needs
(`services/tenant_assurance.rs`):

| Level | Proven by | Marginal cost |
| --- | --- | --- |
| `email_verified` | possession of an inbox | zero |
| `domain_verified` | DNS TXT in the company's zone (ADR-007) | zero |
| `business_verified` | a KYB vendor checking registration documents | per-check + subscription |

**Twilio does not sell KYB** — they use Persona for their own. What Twilio
offers is A2P 10DLC brand registration (messaging compliance, and per ADR-009
it follows whoever owns the sending account) and Verify (phone possession,
which our own OTP already does). So there is nothing to leverage there, and
the vendors that do this are Persona, Stripe Identity and Sumsub.

At roughly USD 250/month, that is not a pre-revenue expense. **`KycProvider`
defaults to `Disabled`**, and the ladder tops out at `domain_verified`.

The seam is built anyway, for two reasons. Retrofitting an assurance concept
into authorisation checks that never had one produces two competing notions of
"verified"; deciding the shape while there are three call sites is cheap.
And every capability the product needs is reachable at `domain_verified` — a
test asserts this — so a disabled provider is not an outage. Enabling a
provider later *adds* a gate on the top rung; it does not revoke a grant that
existing tenants already earned.

For B2B SaaS, domain verification carries most of the weight anyway:
controlling `acme.com`'s DNS is a strong claim to acting for Acme, and it also
answers §5's question about granting the first `tenant_admin`.

---

## 6. Open questions

> **PARTLY ANSWERED (2026-07-25):** the assurance ladder in §5 is the
> mechanism. The signup's first user becomes `tenant_admin` at
> `email_verified`, which is enough to use the console and nothing else;
> capabilities that matter wait for `domain_verified`. Still open: whether a
> free-mail signup (gmail.com) should be able to reach `domain_verified` on a
> company domain it later claims.

> **Open:** Whether operator override on tenant configuration should require a
> reason string recorded in the audit trail. Leaning yes; deferred until Gate C
> audit logging lands.
