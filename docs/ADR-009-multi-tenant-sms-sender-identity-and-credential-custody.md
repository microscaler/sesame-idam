# ADR-009: Multi-tenant SMS sender identity & credential custody

> **Status:** ACCEPTED (2026-07-24) — Phase 1 + Phase 2 implemented
> **Deciders:** Platform (Sesame-IDAM), Microscaler product teams
> **Related:** [ADR-004](./ADR-004-platform-tenant-provisioning.md) (platform
> tenancy), [ADR-006](./ADR-006-shared-signing-keys-for-ha.md) (secret custody
> via the backend chain), [ADR-008](./ADR-008-authentication-assurance-passkey-first-mfa.md)
> (SMS is a RESTRICTED factor; tenant SMS opt-in), the A3 abuse/toll-fraud
> controls (`services/abuse_guard.rs`), and the SMS provider slice
> (`services/sms.rs`). Full design:
> [DESIGN-multi-tenant-sms-sender-identity.md](./DESIGN-multi-tenant-sms-sender-identity.md).

---

## 1. Context

SMS is now a real delivery channel (Twilio provider + purpose-gated send,
cost-restricted to registration/password-reset by default — the A3 slice).
But "which Twilio account pays and sends?" is unanswered, and in a
SaaS-of-SaaS platform (ADR-004) it has more than one right answer:

- **Platform-level identity operations** — onboarding a tenant, provisioning
  a tenant environment, recovering access *to the Sesame platform* — are
  Sesame's relationship and liability.
- **A tenant's own end-users, inside the tenant's application** — their
  registration, password reset, phone re-verification — are the *tenant's*
  relationship, brand, and bill.

Today's `sms.rs` reads a single process-wide Twilio credential from env. That
cannot express per-tenant senders, cannot bill the right party, and would let
one credential's spend or a toll-fraud burst harm everyone. This ADR fixes the
sender-identity and credential-custody model before SMS is used for anything
beyond dev.

The interim `SMS_OPTED_IN_TENANTS` allow-list (ADR-008) and the single global
spend ceiling (A3) are placeholders this ADR supersedes.

---

## 2. Decisions

### 2.1 Sender = relationship owner = billing party

The account that sends (and pays for) an SMS is **whoever owns the
relationship the message serves**, decided server-side and never derived from
tenant-supplied input (confused-deputy prevention):

| Scenario | Account |
|---|---|
| Tenant account registration / owner verification | **Sesame (platform)** |
| New environment registration for a tenant | **Sesame** |
| Tenant owner/admin recovery (access to the Sesame console) | **Sesame** |
| Platform operator MFA / break-glass | **Sesame** |
| End-user registration on a tenant's app | **Tenant** |
| End-user password reset | **Tenant** |
| End-user phone change / re-verification | **Tenant** |
| End-user login MFA / step-up (opt-in, discouraged) | **Tenant** |
| End-user account recovery | **Tenant** |

The dividing question is *which console/app is being authenticated into*.
Tenant-**owner** recovery is platform-billed because it restores access to the
tenant on the platform, not to the tenant's application.

### 2.2 Resolution key is `(tenant, environment, purpose)`

Sender selection is a server-side function
`resolve_sms_sender(tenant, environment, purpose) → SmsSender` returning the
account, the From / Messaging-Service identity, and the applicable caps. Not
purpose alone: a tenant's staging and prod may use different senders, and the
sender's brand identity is per-tenant even when platform-billed.

### 2.3 Three-tier credential custody — prefer *no* custody

1. **Platform account** — one credential set, custodied like the ADR-006
   signing keys: born in the secret backend (OpenBao / GCP SM), delivered by
   ExternalSecret, mounted, never in tenant data or git.
2. **Tenant account — Twilio Connect (PREFERRED)**: the tenant authorizes
   Sesame; **Twilio bills the tenant directly** (confirmed 2026-07-24, Twilio
   Connect docs); Sesame holds only a revocable connection reference (the
   connected `AccountSid`), never the tenant's auth token. This eliminates
   tenant credential custody entirely.
3. **Tenant account — envelope-encrypted raw credentials (FALLBACK)**: for
   tenants who supply their own SID/token, store the token encrypted with a
   per-tenant data key wrapped by a KEK in the backend; decrypt in-process at
   send time; never log plaintext. Scales to thousands where per-tenant k8s
   Secrets never would.

Twilio **Subaccounts** under Sesame's master are explicitly NOT adopted for
tenant billing (they bill the master) — reserved only for a future
platform-aggregated resale model.

### 2.4 Layered, per-owner spend ceilings

The single global A3 ceiling becomes a hierarchy, all enforced before send:

- **per-recipient** window/day caps (existing A3) — anti-bombing;
- **per-tenant** daily spend ceiling — one tenant cannot exhaust another;
- **platform** daily spend ceiling — bounds platform-billed toll fraud;
- **per-provider-credential** ceiling — a compromised credential is bounded.

Ceilings are keyed on the resolved billing owner, so tenant and platform
budgets are independent.

### 2.5 Fallback is email-only or hard-block — never silent subsidy

When a tenant-billed purpose has no usable tenant sender (no Connect
authorization, no valid stored credential), Sesame falls back to **email OTP**
or refuses — it never charges the platform account for a tenant's end-user.

### 2.6 Compliance follows the brand, not the bill

A2P 10DLC campaign registration (US), sender-ID registration, and STOP/opt-out
handling are per *sending brand*. A tenant sending under its own brand owns its
campaign registration even under Connect/custody. Sesame records opt-out state
per (sender, recipient) and honours it before every send.

### 2.7 Validate on store; rotate/revoke first-class; meter everything

Tenant credentials/connections are validated (a live credential check) before
they are trusted; are revocable and rotatable per tenant; and every send is
metered per (owner, tenant, environment, purpose, cost) for chargeback and as
the primary toll-fraud signal.

---

## 3. Consequences

**Positive**

- Correct billing attribution; a tenant's SMS spend and fraud are contained to
  the tenant.
- Preferred path (Connect) removes tenant secret custody — the least-liability
  option for Sesame.
- Per-owner ceilings turn one global blast radius into isolated ones.
- Cleanly extends ADR-004 (platform vs tenant) and ADR-006 (backend custody).

**Negative / follow-up**

- New per-tenant SMS config surface (schema + platform-admin API + validation).
- Connect onboarding is a tenant-facing OAuth-style ceremony to build.
- Envelope encryption requires a KEK in the backend and a decrypt-at-send path.
- Metering/chargeback pipeline is new work.

---

## 4. Open questions

> **DECIDED (2026-07-24, lean dogfood):** Twilio Connect is **required for
> external tenants** (no raw-credential custody). Envelope-encrypted raw
> credentials (Tier 2) are permitted for **dogfood tenants only** (hauliage,
> PriceWhisperer) so internal onboarding isn't blocked on the Connect
> ceremony. This keeps Sesame out of external-tenant token custody entirely
> while unblocking dogfood.

> **DECIDED (2026-07-24):** A config row **per `(tenant, environment)`**, as
> the design doc proposed. Staging and prod are genuinely different senders
> with different numbers and separate A2P/10DLC registration; a single row
> would have coupled their ceilings and their revocation.

> **DECIDED (2026-07-25): Twilio Connect only. Single provider, deliberately.**
>
> The question was whether to support tenants who already use Brevo, Vonage,
> MessageBird or a regional carrier. Investigating it surfaced something that
> changes this ADR's central argument rather than just its schema.
>
> §2.3 argues Sesame should hold no tenant credentials, because **Twilio
> Connect** lets a tenant delegate access while Twilio bills them directly.
> No other major provider offers an equivalent. With Brevo or Vonage a tenant
> must hand over an API key — so supporting them would make **envelope custody
> the normal path rather than the dogfood-only fallback**, promoting the KEK
> and its rotation runbook from contingency to load-bearing, and turning
> "prefer Connect" from a principle into a footnote about one vendor.
>
> Adding a second provider would therefore not be a feature; it would be a
> decision to start holding other companies' secrets as routine business.
> That is a materially different product, and not one worth becoming by
> accident while chasing a checkbox.
>
> **So: Twilio Connect is the only tenant-billed path.** The custody model in
> §2.3 stands as written, envelope custody remains the dogfood exception it
> was designed to be, and Sesame holds no external tenant's credentials.
>
> **The commercial edge this creates**, which sales will meet before
> engineering does: a prospect already standardised on another SMS provider
> must open a Twilio account to use tenant-billed SMS, or fall back to
> platform-billed sends and email OTP. That is a real objection and it should
> be answered honestly rather than discovered mid-deal.
>
> **Revisit when** a prospect worth the change refuses Twilio. At that point
> the work is: a provider-neutral credential shape (the schema currently
> speaks Twilio — `account_sid`, `connected_account_sid`,
> `messaging_service_sid`), an actual `SmsProvider` trait (there is none today,
> just `send_twilio` and `send_mock` in `services/sms.rs`), and — the part
> that matters — a conscious decision to accept custody of tenant credentials
> as a standing liability, with the KEK rotation story finished first.
>
> Sender resolution (`sms_sender.rs`) is already provider-agnostic in shape,
> so none of that is a rewrite. It is a decision, not a refactor.

---

## Sources

- [Twilio Connect — billing of connected accounts](https://www.twilio.com/docs/iam/connect)
- [How is billing handled with Twilio Connect?](https://support.twilio.com/hc/en-us/articles/223182588-How-is-billing-handled-if-I-build-an-application-using-Twilio-Connect-)
- [Twilio Subaccounts (billed to the master account)](https://www.twilio.com/docs/iam/api/subaccounts)
