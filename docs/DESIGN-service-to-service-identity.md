# Design: service-to-service identity — API key, mTLS, or neither

> **Status:** PROPOSED (2026-07-25)
> **Prompted by:** "the API key was originally intended for service-to-service
> calls — is that the right plan, or should we consider mTLS?"

---

## 1. What is actually true today

The question assumes a choice between two options, one of which is in place.
Neither is:

- `ApiKeyHeader` is declared in all six service specs, at document level, and
  is attached to **no operation** and used by **no internal caller**.
- `identity-login-service → authz-core` sends `content-type` and `x-tenant-id`.
  No credential. Plaintext `http://authz-core:8080`.
- `identity-login-service → identity-session-service` (JWKS) is plaintext too,
  recorded in Gate A4 as a "known plaintext hop … acceptable inside the mesh
  for staging". There is no mesh.

**So service-to-service calls are currently unauthenticated and unencrypted.**
The API-key plan was written down and never carried through. We are not
choosing between two designs; we are choosing what to build.

### The sharper problem underneath

`authz-core` is asked "what may this principal do" with the tenant supplied as
a **request header**. If any workload that can reach the pod may set that
header freely, then the authorization service will answer questions about any
tenant for any caller — a cross-tenant oracle one layer below the boundary
ADR-011 §2.2 is careful about at the edge.

Whether that is exploitable depends on `authz-core`'s own checks, which this
document does not assume. It needs verifying, and it is the reason S2S identity
matters here rather than being hygiene.

---

## 2. Three concerns wearing one name

"API key" is doing three unrelated jobs in conversation. They have different
right answers and should not share a mechanism.

| # | Question | Principal | Right mechanism |
| --- | --- | --- | --- |
| 1 | Which **workload** is calling? | a pod | mTLS / workload identity |
| 2 | Which **operator** is acting? | a human or CI job | OIDC identity with attribution |
| 3 | Which **tenant's backend** is calling? | a customer's service | bearer credential (`client_credentials`) |

**#3 must stay a bearer credential.** A tenant cannot hold a certificate from
our cluster's CA — ADR-011's airgap says they have no relationship with our
infrastructure. That is the `client_credentials` grant in `auth_token.rs`, and
it is a *product* surface, not infrastructure plumbing. It is fine as it is.

The question is really about #1 and, unexpectedly, #2.

---

## 2a. The BFF case, which is the one that actually matters

A BFF calling a backend is the motivating example, and it does not fit the
table above cleanly — because a BFF is not a peer service. **It acts on behalf
of an end user.** Two identities are in play on every call:

- *which service is calling* (the BFF), and
- *on whose behalf* (the signed-in user).

Answering only the first is the trap. A BFF authenticated by its own
credential, passing a user or tenant identifier as a **header**, means the
backend trusts an attacker-supplied identifier from a caller that is
legitimately authenticated. Any user of the BFF can then act as any other, and
the backend's logs will show a perfectly valid caller doing it.

This is not hypothetical here. It is exactly the shape of the existing
`x-tenant-id` header on the `authz-core` call — the same pattern ADR-011 §2.2
removes at the edge by taking the tenant from the token instead of the path.
One layer down, we reintroduced it.

**So the rule for BFF → backend is: the caller proves itself, and the user
comes from the user's own token.** Never from a header, never from a body
field. The backend derives the subject from a credential the BFF could not
have forged.

The mechanism for that already exists in this codebase: RFC 8693 token
exchange with an `act` (actor) claim, in `controllers/auth_token.rs`
(`ActorClaim`, `can_delegate`). The delegated token says "user U, acted for by
service S", so the backend gets both identities from one verified artefact and
neither is caller-asserted. That machinery is built and unused for this path.

### Whose BFF is it?

The answer to "mTLS or bearer" then depends on which side of the ADR-011
airgap the BFF sits:

- **Our own BFF**, inside our trust domain → workload identity (mTLS) for the
  caller half, plus a delegated user token for the subject half.
- **A tenant's BFF** — hauliage's, for instance, which is a *tenant* of Sesame
  — cannot hold a certificate from our CA, by the same airgap that says a
  tenant holds no platform credential. It authenticates with
  `client_credentials`, and still carries a delegated user token.

Both cases keep the same rule about the subject. Only the caller half differs.

### "Who says the call is really from the BFF?"

Nothing, today. And this is the question that decides the design, because a
delegated user token does not answer it either.

A user token proves **the user consented**. It says nothing about **who is
presenting it**. It is a bearer credential: whoever holds the bytes can use
them. So a service that obtains one — from a log line, an error report, an
SSRF against the BFF, a stolen backup, a compromised sidecar — calls the
backend and is *indistinguishable from the real BFF*. So does anything holding
a copied API key. Both credentials answer "is this string valid", never "are
you the workload I think you are".

Only **proof of possession bound to a workload identity** answers it. Under
SPIFFE/mesh issuance, a pod's identity derives from its ServiceAccount and
platform attestation, and its private key never leaves it. To impersonate the
BFF an attacker must be able to *schedule a pod as that ServiceAccount in that
namespace* — a far higher bar than reading a secret, and one that leaves
traces in the control plane.

That is the difference in one line: **a secret can be copied; a workload
identity has to be forged in the cluster's control plane.**

Three limits worth stating plainly, so the guarantee is not oversold:

- **mTLS defends against impersonation, not against compromise of the genuine
  caller.** RCE inside the real BFF gives the attacker the real identity.
  Nothing at this layer helps; that is what the subject half and per-route
  policy are for.
- **Which is why the delegated user token still matters.** Even a legitimate
  BFF must not be able to assert arbitrary users. mTLS bounds *which service*;
  the delegated token bounds *which user*. Neither substitutes for the other,
  and this is the whole reason to do both rather than pick one.
- **NetworkPolicy is topology, not identity.** It stops a fake service that
  cannot reach the pod; it does nothing about one that lands somewhere allowed.
  Useful, cheap, and not an answer to this question.

So the ranking for the caller half is not close:

| Mechanism | Stops a copied credential? | Stops a fake stood-up service? |
| --- | --- | --- |
| Shared API key | no | no |
| User's bearer token alone | no | no |
| NetworkPolicy | n/a | only if it cannot reach the pod |
| mTLS with workload identity | yes | yes, absent control-plane compromise |

---

## 3. On mTLS for workload identity (#1)

**Recommended, with one caveat that matters.**

Why it beats a shared API key:

- A key is a bearer secret: anyone who reads it — a log line, an env dump, a
  SOPS file, a stack trace — can replay it. A client certificate requires the
  private key, which never leaves the workload.
- Rotation is automatic under SPIFFE/mesh issuance. A shared key is rotated by
  a human, which means in practice it is rotated after an incident.
- It encrypts the hop, which closes the BR-1c plaintext JWKS gap as a side
  effect rather than as separate work.
- It needs no application code. The identity arrives below the app.

**The caveat: mTLS authenticates the caller, not the call.** It answers "this
is identity-login-service", not "identity-login-service may ask authz-core
about tenant X". Deployed without an authorization policy, mTLS produces a
cluster-wide "any workload may call anything" credential — which is precisely
what a shared API key already is, at higher operational cost.

So mTLS is only an improvement when paired with a per-caller policy: which
service identity may reach which route. That is the part worth designing; the
transport is the easy half.

### Options, given the current stack

Envoy Gateway is deployed at the edge; there is no mesh.

- **Service mesh (Linkerd).** mTLS by default, automatic rotation, no app
  changes, per-route authorization policies. Lightest of the meshes. New
  operational surface to run and debug.
- **cert-manager certificates + Envoy.** No mesh, but issuance, distribution
  and rotation become ours, which is the part meshes exist to do.
- **Do nothing yet, close the gap differently.** NetworkPolicy (Gate B4, open)
  restricts *who can reach* a pod, which is not identity but removes the
  "anything in the cluster" premise cheaply and is already on the tracker.

---

## 3a. Every inter-service call asks three questions, not one

RERP is the clarifying example: an accounting **invoice** module calls a
**document generation** module to render a PDF. "Is that a valid call?" turns
out to be three separate questions, and mTLS answers only the first.

| # | Question | Answered by | If missing |
| --- | --- | --- | --- |
| 1 | Is the caller really the invoice service? | mTLS / workload identity | anything on the network can render documents |
| 2 | On whose behalf, and for which tenant? | delegated user token (`act` claim) | a valid caller renders any tenant's invoice |
| 3 | May *that* subject touch *this* invoice? | authorization + RLS on the object | a valid caller and a valid user still reach another customer's data |

Getting 1 without 2 and 3 is the dangerous state, because it *feels* secure:
every call is mutually authenticated, the dashboards are green, and the
document service will still happily render invoice `12345` for whoever asks.
Caller identity is not authority over data.

Question 3 is the one that bites hardest in a product like RERP, where the
interesting objects belong to tenants. `doc-gen` must not render an invoice
because *the invoice service asked*; it must render it because the subject in
the delegated token is entitled to that invoice — which is an authorization
decision over an object, and in this stack that means the ADR-005 RLS contract
carrying the tenant through to the query, not a check the caller performs on
its own say-so.

Two corollaries worth stating, because they are easy to get backwards:

- **A service is not a principal for data access.** `invoice-service` has no
  tenant. It acts for subjects who do. Any design where a backend module holds
  standing access to all tenants' data and is trusted to filter correctly has
  moved the entire tenancy boundary into that module's correctness.
- **Per-caller policy is about routes, not rows.** "invoice-service may call
  `POST /documents/render`" is question 1's companion and belongs in mesh
  policy. It says nothing about *which* document, which is question 3.

---

## 3b. Does k3s with ServiceAccounts prove it out for GCP/AWS/Azure/self-hosted?

**Yes for the mechanism, with two caveats that are worth knowing before it is
called proven.**

### Why it ports

The portable substrate is the **projected ServiceAccount token**: audience-
scoped, pod-bound, short-lived, auto-rotated, and signed by the cluster. That
primitive is identical on k3s, GKE, EKS, AKS and a self-hosted cluster —
it is Kubernetes, not a cloud feature.

SPIFFE builds workload identity on exactly that, which is what makes it the
right target for a system that intends to run anywhere. A SPIFFE ID is

```
spiffe://<trust-domain>/ns/<namespace>/sa/<serviceaccount>
```

with no cloud in it. The policy "`ns/rerp/sa/invoice` may call
`ns/rerp/sa/docgen`" is written once and means the same thing on a laptop k3s
and in a customer's own data centre. That is the property you are asking
about, and it is real.

### Caveat 1 — a ServiceAccount token is not proof of possession

This is the distinction that decides whether the exercise is meaningful. A
projected SA token is still a **bearer** credential: copy it out of a pod and
it works from anywhere until it expires. Using SA tokens directly as the
service-to-service credential rebuilds the problem this document exists to
solve, with better rotation.

The SA token's job is **attestation** — evidence used *once* to obtain an
identity — not authentication on every call. What proves possession per
connection is the X.509 SVID and its private key, which never leaves the
workload. So:

> ServiceAccount answers *who deserves an identity*.
> mTLS answers *who is on the other end of this connection*.

A proof-of-concept that stops at "we pass the SA token" has not proved the
thing that matters.

### Caveat 2 — node attestation is the part that changes per platform

Workload attestation (which pod, which ServiceAccount) is the same everywhere.
**Node** attestation is not:

| Platform | Typical node attestor |
| --- | --- |
| k3s / self-hosted | `k8s_psat` (projected SA token) |
| GCP | `gcp_iit` (instance identity token) |
| AWS | `aws_iid` (instance identity document) |
| Azure | `azure_msi` (managed identity) |

So the SPIFFE IDs, the policies and the application code port unchanged; the
attestor configuration is per-platform. That is a deployment concern, not a
redesign — but it means "it works on k3s" proves the model, not the whole
production posture.

A third thing k3s will not exercise: **trust-domain federation**. One cluster
means one trust domain. Multi-cluster or multi-cloud requires federating
trust domains, which has its own failure modes and should not be assumed
proven by a single-cluster demo.

### Don't confuse this with cloud "Workload Identity"

GKE Workload Identity, EKS IRSA and AKS Workload Identity federate a k8s
ServiceAccount to a **cloud IAM** identity. That is for reaching *cloud
resources* — a bucket, a KMS key, Secret Manager — and it is genuinely useful
(it is how `SMS_CREDENTIAL_KEK` should eventually live in a cloud KMS rather
than a SOPS file).

It is **not** service-to-service identity inside the cluster, and it does not
port to self-hosted, where there is no cloud IAM to federate with. Same input
(the ServiceAccount), different problem. Keeping them separate matters,
because a self-hosted deployment must not lose service identity just because
it has no cloud provider.

### What BRRTRouter already has, and which half it is

BRRTRouter carries roughly 1,500 lines of SPIFFE support
(`src/security/spiffe/`): trust-domain and SPIFFE ID validation, a JWKS cache
with TTL and refresh, `kid` lookup, signature verification, revocation. That is
real work and it is in the right vocabulary.

It is **JWT-SVID** — the module contains no `x509`, `mtls` or `client_cert`.
That matters, because the two SVID forms answer different questions:

| | JWT-SVID | X.509-SVID + mTLS |
| --- | --- | --- |
| What it is | a signed bearer token carrying a SPIFFE ID | a certificate whose private key never leaves the workload |
| Proves | this token was issued to that identity | *this connection* is that identity |
| If copied | replayable by whoever holds it | useless without the key |

So the existing work is the **identity and authorization half**: parse a SPIFFE
ID, verify it was issued by a trusted domain, decide whether that identity may
call this route. What it does not do — and arguably should not — is prove who
is on the other end of the socket. A stolen JWT-SVID replays exactly like the
user token described in §2a.

This is a sensible division rather than a shortfall. Proof of possession
belongs in the transport, which in Kubernetes means the mesh or a sidecar, not
the application. The two compose:

- **Channel:** mesh mTLS with X.509-SVIDs proves the peer.
- **Assertion:** JWT-SVID (or the mesh's verified peer identity) tells the
  application *which* identity, so it can apply per-route policy.

With a mesh in place the peer identity usually arrives as a sidecar-set header
(Envoy's `x-forwarded-client-cert`). Trusting that header is legitimate **only**
because the sidecar is unavoidably in-path — the same topology assumption as
the `X-Forwarded-For` trust in the Gateway's `ClientTrafficPolicy`, and it
fails the same way if a pod is reachable around the proxy.

Without a mesh, JWT-SVID alone is still an improvement on a shared API key —
short-lived, per-workload, audience-scoped — provided the audience binds the
token to the **callee**, so a token captured by one service cannot be replayed
against another. That binding is what limits the blast radius of the bearer
weakness, and it should be treated as mandatory rather than optional.

### What a proof-of-concept should actually demonstrate

1. Two services in k3s with distinct ServiceAccounts get distinct SVIDs.
2. Calls between them are mTLS, with certificates rotated automatically and no
   secret in any manifest, env var or SOPS file.
3. A **policy denial**: a third workload with a valid identity is refused on a
   route it is not permitted to call. Proving allow without proving deny
   demonstrates nothing.
4. A **forged-caller attempt**: standing up a pod that tries to present
   another service's identity fails, and fails visibly.
5. The subject half end to end: a delegated token carried through, and a
   request for another tenant's object refused at the data layer even though
   caller and user are both entirely valid.

Items 3 to 5 are where the design is actually tested. 1 and 2 are the parts
that demo well.

---

## 4. The finding: the platform admin key has no attribution (#2)

`X-Platform-Admin-Key` is a single static shared secret. Every operator and
every provisioning worker presents the same string.

ADR-011 §2.4 promises that operator action on a tenant is "audited as an
operator action, distinctly from the tenant doing it themselves". With a shared
key that promise cannot be kept: the audit trail can only ever record *someone
who had the key*. It cannot say which operator, and revoking a compromised key
revokes every operator and worker at once.

mTLS does not fix this, because operators are people and CI jobs, not
workloads. The answer is a real operator identity — OIDC-authenticated humans,
short-lived tokens, per-identity revocation — with the shared key surviving
only for unattended provisioning workers, where a workload identity is in fact
the right model.

This is a gap in an ADR written today, found by asking the same question that
found the others: *who concretely does this?*

---

## 5. Recommendation

1. **Do not build an API-key scheme for S2S.** It is the weakest of the
   options and would need replacing; the only reason to reach for it is that it
   is already half-written in the specs, which is not a reason.
1a. **Fix the subject half first — it is cheaper and matters more.** For every
   BFF → backend call, derive the user from a delegated token (RFC 8693, the
   `act` claim machinery already in `auth_token.rs`) rather than a header. A
   caller-asserted subject is a privilege escalation regardless of how well the
   caller itself is authenticated, so this is worth more than any amount of
   transport work.
2. **Close the premise cheaply first:** NetworkPolicy (Gate B4) so "reachable
   from anywhere in the cluster" stops being true while the identity work is
   designed.
3. **Verify the `authz-core` tenant-header question** before anything else. If
   it trusts the header without checking the caller, that is today's problem,
   not next quarter's.
4. **Target mTLS via a mesh for workload identity**, with per-caller
   authorization policy designed alongside — not transport alone.
5. **Separately, give operators individual identities.** Different problem,
   different mechanism, and the one that a shared key currently answers worst.
6. **Leave `client_credentials` alone.** It is the correct mechanism for the
   one caller that genuinely cannot hold a certificate.

---

## 6. Open questions

> **Open:** Does `authz-core` verify the caller before honouring `x-tenant-id`,
> and does any BFF pass a user identifier the same way? Determines whether the
> subject-half work is urgent or merely untidy. Worth answering first: it is a
> read of two call sites.

> **Open:** Mesh or cert-manager. Depends on appetite for operational surface;
> Linkerd is the lightest path to mTLS-by-default with rotation handled.
> Note this is now a question about the *channel* only — BRRTRouter already
> covers SPIFFE ID validation, so the application half is not starting from
> zero.

> **Open:** Does BRRTRouter's JWT-SVID validation bind the audience to the
> callee? If not, a token captured from one service replays against any other,
> which removes most of the benefit over a shared key.

> **Open:** Whether provisioning workers keep a shared key once operators have
> individual identities, or become workload identities under the same mTLS
> scheme. The latter is more coherent and removes the last shared secret.
