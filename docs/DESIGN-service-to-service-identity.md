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

> **Open:** Whether provisioning workers keep a shared key once operators have
> individual identities, or become workload identities under the same mTLS
> scheme. The latter is more coherent and removes the last shared secret.
