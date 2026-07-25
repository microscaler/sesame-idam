# Design: self-service tenant registration

> **Status:** PROPOSED (2026-07-25)
> **Related:** [ADR-004](./ADR-004-platform-tenant-provisioning.md) (platform
> provisioning), [ADR-007](./ADR-007-tenant-domain-verification.md) (domain
> verification), [ADR-010](./ADR-010-frontend-architecture-hosted-auth-and-client-sdk.md)
> (frontend), [ADR-011](./ADR-011-platform-tenant-authority-separation.md)
> (authority domains and the assurance ladder).
> **Task:** 32

---

## 1. What this has to do

Take a stranger from the brochure site to a working tenant console, without
letting the front door become a way to create platform state, enumerate
customers, or acquire authority nobody granted.

The journey:

```
brochure CTA → signup form → email possession → tenant activated
             → first admin signed in → console (assurance: email_verified)
             → domain verification (ADR-007) → capabilities unlocked
```

---

## 2. Design

### 2.1 Public signup creates a *lesser* thing than provisioning

ADR-011 §2.4 keeps tenant creation platform-scoped. Self-service does not
overturn that; it creates something weaker.

`POST /signup/tenant` is the only public path that creates a tenant, and the
tenant lands in **`provisioning`** status — which `TenantService::require_active`
already refuses for authentication. So a tenant that has been asked for but not
confirmed exists as a row and nothing else: nobody can sign into it, and it
grants no capability.

Activation (`provisioning → active`) is an already-validated transition, and
happens only when email possession is proven. **The gate is a state machine
that predates this feature, not a new check to remember.**

### 2.2 No slug is accepted, so there is no enumeration oracle

The caller supplies a company name. The slug is **derived server-side** and
de-duplicated by suffix.

Accepting a slug would require answering "is this one taken", which is an
oracle for enumerating the platform's customer list — the thing ADR-011 §2.5
prohibits. Deriving it removes the question rather than answering it carefully,
and removes a failure mode from the form at the same time.

Reserved slugs (`admin`, `api`, `platform`, `www`, `sesame`, …) are already
enforced in `TenantService`.

### 2.3 The response is the same whether or not the email is known

`202 Accepted` with `{"status": "verification_sent"}` in every non-malformed
case, including when the address already has an account.

A distinguishable response tells an attacker which addresses are customers.
The person who genuinely owns the address learns the truth from the email they
receive, which is the channel that has already proven possession.

### 2.4 The first user becomes `owner`

The signup's user is created with `owner`, which ADR-011 §2.3 says implies
`tenant_admin`. This answers that ADR's open question about granting the first
administrator on self-service signup.

It is deliberately paired with §2.5 below: `owner` of a tenant at
`email_verified` can administer very little, so the grant is not worth
attacking on its own.

### 2.5 Capabilities wait for the assurance ladder

Completing signup reaches `email_verified` — enough for `UseConsole` and
nothing else (`services/tenant_assurance.rs`). SMS, SSO and production access
require `domain_verified` (ADR-007).

This is what makes an unverified signup safe to allow freely: the front door is
open, but the rooms are locked. A free-mail signup can create a tenant and look
around; it cannot send SMS on anyone's behalf.

### 2.6 Rate limiting is not optional here

Signup creates rows and sends mail on an unauthenticated path. It needs a Gate
A1 budget alongside the existing groups, stricter than `register` because a
tenant is more expensive than a user.

---

## 3. Open questions

> **Open:** Should a free-mail signup (`gmail.com`) be able to reach
> `domain_verified` on a company domain it later claims? Leaning towards
> allowing it — controlling the DNS is the proof, and the signup address is
> incidental — but it deserves a decision rather than a default.

> **Open:** What happens to an abandoned `provisioning` tenant. A sweep after
> some days seems right; the slug should return to the pool.

> **Open:** Whether signup should require the ADR-008 passkey enrolment at
> first sign-in rather than a password. Better security, more friction at the
> worst possible moment for conversion.

---

## 4. Testing obligation

1. A tenant in `provisioning` **cannot** authenticate.
2. Signing up with an address that already exists is **indistinguishable** from
   a fresh signup, in status code, body and timing class.
3. The derived slug never collides, and never lands on a reserved word.
4. A tenant at `email_verified` is **refused** `SendSms` and `ConfigureSso`.
5. The verification token is single-use and expires.

Test 2 is the one that rots quietly: it survives a refactor that adds an
"account already exists" branch for helpfulness.
