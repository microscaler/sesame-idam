# ADR-012: Service identity is layered — caller, subject, object

> **Status:** ACCEPTED (2026-07-25)
> **Deciders:** Platform (Sesame-IDAM), Microscaler product teams
> **Related:** [ADR-005](./ADR-005-first-class-rls-contract.md) (RLS contract),
> [ADR-011](./ADR-011-platform-tenant-authority-separation.md) (authority
> domains). Analysis:
> [DESIGN-service-to-service-identity.md](./DESIGN-service-to-service-identity.md),
> [BRRTRouter/docs/DESIGN-security-scheme-fail-closed.md](../../BRRTRouter/docs/DESIGN-security-scheme-fail-closed.md).

---

## 1. Context

Service-to-service calls today are unauthenticated and unencrypted. The
`ApiKeyHeader` scheme is declared in six specs, attached to no operation and
used by no caller; `identity-login-service → authz-core` sends the tenant as a
plain header with no credential at all.

The design analysis established that "API key or mTLS?" is the wrong shape of
question, because a single mechanism cannot answer what an inter-service call
actually raises. This ADR fixes the model so the individual pieces of work
have something to be correct against, and sequences them.

---

## 2. Decision

### 2.1 Three questions, three mechanisms, no substitutions

Every inter-service call — BFF to backend, RERP invoice to document
generation, login to authz-core — must answer all three:

| # | Question | Mechanism | Failure if absent |
| --- | --- | --- | --- |
| 1 | Which **workload** is calling? | SPIFFE identity; proof of possession via mTLS | anything on the network can call anything |
| 2 | On **whose behalf**? | delegated user token, RFC 8693 `act` claim | a valid caller acts as any user |
| 3 | May that subject touch **this object**? | authorization + RLS (ADR-005) | a valid caller and valid user reach another tenant's data |

**None substitutes for another.** The dangerous state is 1 without 2 and 3,
because it presents as secure: every call mutually authenticated, dashboards
green, and the document service still renders any tenant's invoice for whoever
asks.

### 2.2 A service is not a principal for data access

`invoice-service` has no tenant. It acts for subjects who do. No backend may
hold standing access to every tenant's rows on the strength of its own
identity and be trusted to filter correctly — that moves the tenancy boundary
into one module's correctness, where it cannot be audited or tested as a
boundary.

Concretely: the tenant reaching a query comes from the delegated token via the
ADR-005 RLS context, never from a header the caller set.

### 2.3 Caller-asserted identifiers are prohibited

No service may accept a tenant, user, or organisation identifier from a peer as
a header, query or body field and treat it as authoritative. This is ADR-011
§2.2 restated one layer down, and the existing `x-tenant-id` header on the
authz-core call is the standing violation.

Where a caller must indicate *scope* (which of the subject's tenants, say), the
value is a **selection within** what the token already authorises, and must be
validated against it — never a substitute for it.

### 2.4 The subject half comes first

Ordering is deliberate: **question 2 before question 1.**

A caller-asserted subject is privilege escalation regardless of how well the
caller is authenticated, so it stays exploitable no matter how good the
transport becomes. Transport work is also the more expensive and the more
easily deferred. Fixing the subject half is cheaper and removes the larger
risk, so it goes first even though it is the less visible improvement.

### 2.5 JWT-SVID now, mTLS when the trigger fires

BRRTRouter already implements SPIFFE **JWT-SVID** validation — trust domain,
JWKS cache, `kid` lookup, signature verification, revocation. That is the
identity-and-authorization half and it is genuinely useful.

It is a **bearer** credential and does not prove who holds the connection. We
accept that for now, with one mandatory condition:

> **The JWT-SVID audience MUST bind to the callee.** A token captured from one
> service must not be replayable against another. Without this the scheme is a
> shared API key with better ergonomics, and the compensating control is gone.

**Trigger for adopting mTLS** (any one):

- a workload we do not operate runs in the same cluster;
- an internal call carries data that would be reportable if disclosed;
- the first production deployment outside a single trusted cluster;
- trust-domain federation becomes necessary (multi-cluster or multi-cloud).

Naming the trigger now avoids both premature mesh adoption and the more likely
failure of never revisiting it.

### 2.6 Mesh versus cert-manager stays open, and that is fine

The channel mechanism is deliberately **not** decided here. It is an
operational-appetite question, not an architectural one: the SPIFFE IDs,
per-route policies and application code are identical either way, because
§2.5 puts identity in the application and possession in the transport.

Deferring it costs nothing, because nothing above depends on the answer.

### 2.7 Portability is a requirement, not a preference

Identity must work identically on k3s, GKE, EKS, AKS and self-hosted. This
rules out building on cloud-IAM workload identity federation (GKE Workload
Identity, EKS IRSA, AKS Workload Identity) for service-to-service: those
federate a Kubernetes ServiceAccount to *cloud IAM* for reaching cloud
resources, and have no equivalent self-hosted.

They remain the right answer for reaching cloud resources — a KMS-held
`SMS_CREDENTIAL_KEK` eventually — which is a different problem with the same
input. The two must not be conflated, or a self-hosted deployment loses service
identity for want of a cloud provider.

The portable substrate is the projected ServiceAccount token: audience-scoped,
pod-bound, short-lived, rotated, and identical everywhere because it is
Kubernetes rather than a cloud feature. Note it is **attestation** — evidence
used once to obtain an identity — and not a per-call credential.

### 2.8 Security schemes fail closed

A declared-but-unconfigured scheme must refuse traffic, never default to a
credential. This is the BRRTRouter uplift, and it applies to Sesame as a
consumer: no service may start believing a scheme is enforced when it is not.

---

## 3. Consequences

**Positive**

- Each piece of work has a defined job, so partial progress is no longer
  mistaken for completion — the failure mode that made "mutually authenticated"
  feel like "secure".
- The mesh decision is deferred without blocking anything.
- Portability is settled before it is expensive to change.

**Negative / follow-up**

- Three mechanisms to keep correct rather than one.
- JWT-SVID's bearer weakness is accepted for a period, mitigated only by
  audience binding — which therefore has to be verified, not assumed.
- Delegated tokens add a hop to call paths that currently forward a header.

---

## 4. Implementation sequence

Ordered by risk removed per unit of effort. Each states what it proves.

| Step | Task | Proves |
| --- | --- | --- |
| 1 | 38 — does authz-core check its caller before honouring `x-tenant-id`? | whether §2.3's standing violation is exploitable today. Two call sites. |
| 2 | 42 — does JWT-SVID bind audience to callee? | whether §2.5's mandatory condition holds. One file. |
| 3 | 39 — delegated user token on BFF→backend | §2.2 and §2.4: subject no longer caller-asserted |
| 4 | 41 — per-operator identity | ADR-011 §2.4's audit promise becomes keepable |
| 5 | 40 — workload identity / mTLS | §2.1 question 1, once a §2.5 trigger fires |

Steps 1 and 2 are reads, not builds, and both can invalidate assumptions the
rest depends on. Neither should be skipped for being small.

---

## 5. Testing obligation

Mirroring ADR-011 §4, because a layered model is only real if each layer is
tested where it fails:

1. A call with **no caller identity** is refused.
2. A call with a **valid caller** but a caller-asserted subject is refused —
   the subject must come from a token.
3. A call with a valid caller **and** a valid subject is refused when the
   object belongs to another tenant.
4. A JWT-SVID issued for service A is **rejected** by service B (audience
   binding).

Test 3 is the one that matters most and is easiest to omit, because reaching it
requires both other layers to be working.

---

## 6. Open questions

> **Open:** Mesh or cert-manager for the channel, when a §2.5 trigger fires.
> Deliberately deferred (§2.6).

> **Open:** Whether provisioning workers keep a shared key once operators have
> individual identities, or become workload identities. The latter is more
> coherent and removes the last shared secret.

> **Open:** Whether `authz-core` should accept a delegated token directly rather
> than being told a tenant at all — which would make §2.3 structural for that
> service rather than a rule to follow. Likely yes; depends on step 1's finding.
