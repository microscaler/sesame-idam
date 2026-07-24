# Design: Multi-tenant SMS sender identity, credential custody & spend control

Status: DESIGN (2026-07-24). Decision record:
[ADR-009](./ADR-009-multi-tenant-sms-sender-identity-and-credential-custody.md).
Builds on: ADR-004 (platform vs tenant), ADR-006 (backend secret custody),
ADR-008 (SMS as RESTRICTED factor, tenant opt-in), the A3 abuse/toll-fraud
controls, and the SMS provider slice (`services/sms.rs`, `services/otp.rs`).

This document is the implementation-level companion to ADR-009. It specifies
the sender-resolution model, the credential-custody tiers in detail, the spend
ceiling hierarchy, compliance handling, the data model, the code seam, and a
phased rollout. It is deliberately provider-aware (Twilio first) but the
sender-resolution contract stays provider-agnostic.

---

## 1. The governing principle

> The account that **sends and pays** for a message is whoever owns the
> **relationship** the message serves — and that decision is made
> **server-side**, never from tenant-supplied input.

Everything else is a corollary. Two actors exist in a SaaS-of-SaaS platform:
the **platform** (Sesame operator) and the **tenant** (a downstream product
like hauliage). A tenant additionally has **end-users** inside its own
application (the B2B2C leg). Messages divide by *which console/app the human is
authenticating into*:

- Authenticating into **Sesame / the platform** → platform relationship →
  platform account.
- Authenticating into a **tenant's application** → tenant relationship →
  tenant account.

## 2. Scenario → account matrix (complete)

| # | Scenario | Authenticating into | Account | Notes |
|---|---|---|---|---|
| 1 | Tenant account registration / owner phone verification | Platform | **Platform** | Onboarding to Sesame |
| 2 | New environment registration for a tenant | Platform | **Platform** | Provisioning op |
| 3 | Tenant **owner/admin** account recovery | Platform (Sesame console) | **Platform** | Restores access *to the tenant on the platform* |
| 4 | Platform operator MFA / break-glass | Platform | **Platform** | Platform staff |
| 5 | Security/billing alert about a tenant's *platform* account | Platform | **Platform** | Platform duty of care |
| 6 | End-user registration on a tenant's app | Tenant app | **Tenant** | Tenant's user + brand |
| 7 | End-user password reset | Tenant app | **Tenant** | " |
| 8 | End-user phone number change / re-verification | Tenant app | **Tenant** | " |
| 9 | End-user login MFA / RFC 9470 step-up | Tenant app | **Tenant** | Opt-in only; email OTP preferred (cost) |
| 10 | End-user account recovery | Tenant app | **Tenant** | " |

The subtle case is **#3**: it feels tenant-flavoured but is platform-billed,
because losing the owner's access is losing access to the tenant *on Sesame*.

The default **cost policy** (from the A3 slice) still applies on top: SMS is
enabled only for `registration` and `password_reset` purposes unless a
deliberate per-environment override adds more. Rows 9/10 are the ones most
often left email-only for cost.

## 3. Sender resolution

Selection is a pure, server-side function:

```
resolve_sms_sender(tenant, environment, purpose) -> SmsSender
```

where

```
SmsSender {
    billing_owner: Platform | Tenant(tenant_id),
    provider:      Twilio | ...,
    credential:    PlatformSecret
                 | TenantConnect { connected_account_sid }
                 | TenantEnvelope { key_ref },
    from:          MessagingServiceSid | FromNumber,
    caps:          resolved spend/rate ceilings for the billing owner,
    compliance:    { campaign_ref, requires_opt_in_check: bool },
}
```

Key points:

- **`purpose → billing_owner` is a fixed server-side table** (matrix §2). The
  request cannot influence it. A tenant end-user flow *cannot* be made to
  resolve to the platform account.
- **`(tenant, environment)` selects the tenant sender row.** A tenant's staging
  and prod are distinct senders (distinct numbers, distinct campaign
  registration), so the config is keyed per `(tenant, environment)`.
- **Absent tenant sender → fallback.** If a tenant-billed purpose resolves to a
  tenant with no usable sender, `resolve_sms_sender` returns a
  `Fallback(email_only | blocked)` outcome — never the platform credential.

## 4. Credential custody (three tiers)

### Tier 0 — Platform account (single, backend-custodied)

Exactly the ADR-006 pattern: the platform Twilio auth token is born in the
secret backend (OpenBao / GCP Secret Manager), referenced by an
`ExternalSecret`, materialised as a mounted cluster Secret, and read by the
service. Never in the `tenants` data, never in git. One credential, rotated
via the backend.

### Tier 1 — Tenant via Twilio Connect (PREFERRED — no custody)

The tenant authorises Sesame's Connect App. **Twilio bills the tenant's own
account directly** for all usage (confirmed against Twilio Connect billing
docs, 2026-07-24). Sesame stores only the connected `AccountSid` — a revocable
reference, not a secret. Sesame acts on the connected account using **its own**
platform auth token scoped to authorised connections; it never holds the
tenant's auth token.

Why preferred: it removes the entire tenant-credential-custody problem and its
liability. Revocation is the tenant's to control (deauthorise the Connect App).
This is the SMS analogue of Stripe Connect.

Onboarding ceremony (tenant-facing, to be built): platform-admin/tenant-admin
initiates → redirect to Twilio Connect authorise → callback stores
`connected_account_sid` + status on the tenant SMS config → validation send.

### Tier 2 — Tenant raw credentials, envelope-encrypted (FALLBACK)

For tenants who insist on supplying their own SID + auth token (e.g. dogfood
tenants, or tenants not using Connect):

- Generate a per-tenant **data encryption key (DEK)**; encrypt the auth token
  with it (AES-256-GCM). Store ciphertext + nonce in the tenant SMS config row.
- Wrap the DEK with a **key encryption key (KEK)** held in the backend
  (OpenBao transit / GCP KMS). Store only the wrapped DEK (or a KEK key-ref).
- At send time: unwrap DEK via the backend, decrypt the token in-process, use,
  drop. Plaintext never persists, never logs, never leaves the process.
- Rotation is per-tenant (re-encrypt under a new DEK); KEK rotation is a
  backend operation that re-wraps DEKs.

This scales to thousands of tenants — you do **not** mount thousands of k8s
Secrets. It reuses the DB + RLS model already in place.

### NOT adopted — Twilio Subaccounts for tenant billing

Subaccounts bill the **master** account, so they do not achieve tenant-direct
billing. Reserve them only for a hypothetical future where Sesame *resells*
SMS and aggregates billing itself (a different business model).

## 5. Spend ceiling hierarchy

The single global A3 ceiling (`SMS_DAILY_SPEND_CEILING_CENTS`) becomes a
hierarchy, all checked before dispatch, all keyed on the **resolved billing
owner**:

| Ceiling | Scope | Prevents |
|---|---|---|
| per-recipient window/day (existing A3) | one phone number | OTP bombing a single victim |
| **per-tenant daily spend** | one tenant's account | one tenant exhausting its own / others' budgets |
| **platform daily spend** | the platform account | platform-billed toll fraud (rows 1–5) |
| **per-credential daily spend** | one provider credential | a leaked/compromised credential |

Because tenant and platform ceilings are independent keys, a tenant hitting its
ceiling never blocks platform onboarding SMS and vice-versa. Ceilings are
config (per tenant, and a platform default), stored on the SMS config / a
platform settings row. Nearing-ceiling and ceiling-hit both emit audit events
(Gate C threat signals) and can drive alerts.

## 6. Compliance (follows the brand)

- **A2P 10DLC (US) / sender-ID registration**: per *sending brand*. Platform
  onboarding messages register under Sesame's brand/campaign; a tenant sending
  under its own brand (Connect or custody) owns its campaign registration. The
  SMS config records the `campaign_ref` per sender; sends without a registered
  campaign in regions that require one are refused with a clear error, not
  silently attempted.
- **STOP / opt-out**: per (sender, recipient). Sesame records opt-out state and
  checks it before every send; honouring STOP is legally required and also a
  deliverability signal.
- **Quiet hours / locale**: per tenant (config), applied to non-critical
  messages.

## 7. Data model (proposed)

A new per-`(tenant, environment)` SMS config, secret material handled per the
custody tier — never raw in this row:

```
sesame_idam.tenant_sms_config
  tenant_id            varchar   (FK-ish to tenants.slug)
  environment          varchar   (dev|staging|prod|...)
  provider             varchar   (twilio|...)
  custody_mode         varchar   (connect|envelope|platform)
  connected_account_sid varchar  NULL   -- Tier 1 (Connect)
  auth_token_ciphertext bytea    NULL   -- Tier 2 (envelope)
  auth_token_nonce      bytea    NULL   -- Tier 2
  dek_wrapped           bytea    NULL   -- Tier 2 (or a KEK key-ref)
  from_messaging_sid    varchar  NULL
  from_number           varchar  NULL
  campaign_ref          varchar  NULL   -- 10DLC/A2P registration id
  daily_spend_ceiling_cents int
  status               varchar   (active|pending_validation|revoked)
  last_validated_at    timestamptz NULL
  created_at, updated_at timestamptz
  PRIMARY KEY (tenant_id, environment)
```

Platform sender config (single) lives in platform settings / env, not this
table. Opt-out state lives in its own small table keyed (sender, recipient).
All rows are tenant-scoped under the existing RLS contract (ADR-005).

## 8. Code seam

- `services/sms.rs` grows `resolve_sms_sender(tenant, environment, purpose)`
  and `send_sms` takes a resolved `SmsSender` (not env-read credentials). The
  `SmsProvider` trait stays; a Twilio provider gains a `send_via_connect`
  variant (act-on-connected-account) alongside `send_with_credentials`.
- The purpose→billing-owner table is a `const` server-side map — the single
  most security-critical line (confused-deputy prevention).
- The abuse guard's SMS gate (`gate_otp_send`) is extended to consult the
  resolved owner's ceilings (§5) rather than one global key.
- A `services/sms_config.rs` (or org-mgmt surface) owns CRUD + validation +
  Connect onboarding callbacks + rotation, behind platform/tenant-admin auth.
- Test seam unchanged: the mock provider (Redis outbox) already lets e2e read
  codes back; resolution is unit-testable with in-memory config.

## 9. Phased rollout

1. **Sender resolution + platform tier + per-owner ceilings.** Introduce
   `resolve_sms_sender` and the purpose→owner map; platform account custodied
   per ADR-006; replace the single global ceiling with the hierarchy. Tenant
   sends still fall back to email until Tier 1/2 land. (Unblocks correct
   platform-billed onboarding SMS.)
2. **Tenant config schema + envelope custody (Tier 2).** `tenant_sms_config`,
   platform-admin CRUD, validate-on-store, envelope encryption with a backend
   KEK. Enables dogfood tenants (hauliage) to send under their own account.
3. **Twilio Connect (Tier 1).** The onboarding ceremony + connected-account
   sends; make Connect the default/required path for external tenants.
4. **Compliance + metering.** 10DLC/campaign fields enforced; STOP handling;
   per-(owner,tenant,purpose,cost) usage metering + chargeback export + alerts.

## 10. Security considerations

- **Confused deputy** is the headline risk: the purpose→billing-owner mapping
  MUST be server-side and immutable to the request. A tenant end-user flow must
  never resolve to the platform credential.
- **Least custody**: prefer Connect (no token) over envelope (token, encrypted)
  over anything that stores plaintext (never).
- **Blast-radius isolation**: per-owner ceilings + per-credential ceilings mean
  a single leaked credential or a single abusive tenant is bounded.
- **Validate before trust**: never store a tenant credential/connection without
  a live validation; mark `pending_validation` until it passes.
- **Audit everything**: every send carries (owner, tenant, environment,
  purpose, cost, sender) — the chargeback ledger and the toll-fraud signal are
  the same stream.
- **Fallback never subsidises**: absent tenant sender → email or block, never
  the platform account.

## 11. Open questions

- Connect-required vs both-supported for external tenants (leaning
  Connect-required; envelope for dogfood only).
- One config row with an environment dimension vs a row per
  `(tenant, environment)` — this design proposes the latter.
- Non-Twilio / WhatsApp Business providers — trait already allows it; keep
  resolution provider-agnostic; out of scope for the first slice.

## Sources

- [Twilio Connect — billing of connected accounts](https://www.twilio.com/docs/iam/connect)
- [How is billing handled with Twilio Connect?](https://support.twilio.com/hc/en-us/articles/223182588-How-is-billing-handled-if-I-build-an-application-using-Twilio-Connect-)
- [Twilio Subaccounts (billed to the master account)](https://www.twilio.com/docs/iam/api/subaccounts)
