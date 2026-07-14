# P2 — Complete the Authentication Surface

**Target:** Launch 1.0 GA

**Outcome:** users can securely manage their account and complete password, verification, MFA,
social, and passwordless authentication journeys.

## Scope and dependencies

P2 depends on P0 token/session semantics and the delivered password login/register path. GA
requires user self-service/admin lifecycle, TOTP MFA with recovery, email verification, and the
three named social providers. Phone/SMS and passwordless delivery require an approved provider
and abuse-control design; unsupported provider breadth is out of scope.

## Functional requirements

| ID | Requirement |
|---|---|
| FR-P2-001 | Authorized users/admins MUST get, update, search, disable, enable, and delete users within their tenant and role boundary. |
| FR-P2-002 | Disable/delete/credential-reset operations MUST revoke or version-invalidate affected sessions and preserve required audit records. |
| FR-P2-003 | Email verification MUST issue opaque, expiring, single-use tokens and expose resend behavior with enumeration-safe responses. |
| FR-P2-004 | TOTP enrollment MUST require confirmation before activation; recovery codes MUST be one-time, hashed at rest, replaceable, and shown only once. |
| FR-P2-005 | Sensitive operations MUST support step-up authentication and record method/time in the trusted session or token context. |
| FR-P2-006 | Google, Microsoft, and GitHub OAuth/OIDC flows MUST validate state, nonce where applicable, redirect URI, provider issuer, and account-linking rules. |
| FR-P2-007 | Magic-link and email/SMS OTP flows MUST use expiring, single-use capabilities and bind verification to the intended tenant, client, and flow. |
| FR-P2-008 | Account-linking conflicts MUST require proof of control and MUST NOT merge accounts solely because providers return the same email. |
| FR-P2-009 | Password change/reset MUST use the configured password policy and Argon2id storage and MUST invalidate recovery capabilities after use. |

## Non-functional requirements

| ID | Requirement |
|---|---|
| NFR-P2-001 | Login, recovery, verification, and lookup responses MUST resist user/tenant enumeration in status, body, and materially observable timing. |
| NFR-P2-002 | Password, OTP, recovery, and provider callback endpoints MUST have tenant-aware brute-force/rate limits and actionable abuse telemetry. |
| NFR-P2-003 | Provider/email/SMS dependencies MUST have explicit timeouts, idempotency, retry rules, and user-safe degraded behavior. |
| NFR-P2-004 | Secrets and recovery artifacts MUST be encrypted or one-way hashed as appropriate and never appear in logs, traces, or analytics. |
| NFR-P2-005 | Core account and password login flows MUST remain usable when optional social or messaging providers are unavailable. |

## Acceptance criteria

| ID | Observable evidence |
|---|---|
| AC-P2-001 | A user can register, verify email, enroll and verify TOTP, use one recovery code once, and complete a step-up-protected action. |
| AC-P2-002 | Each named social provider completes login and safe account linking; state/nonce replay, callback tampering, and unproven email collision are rejected. |
| AC-P2-003 | Magic-link and OTP capabilities fail after first use, expiry, tenant mismatch, or client/flow mismatch. |
| AC-P2-004 | User lifecycle BDD proves authorized same-tenant behavior and rejection of unauthorized role, cross-tenant, disabled-user, and deleted-user cases. |
| AC-P2-005 | Disable, delete, password reset, and recovery events invalidate the sessions/capabilities specified by policy and emit redacted audit events. |
| AC-P2-006 | Abuse tests prove configured rate limits and enumeration-safe responses; provider-outage tests prove password login remains available. |

## Exit evidence

- Publish the supported-provider matrix, TTL/rate-limit policy, account-linking policy, and
  session invalidation matrix.
- Link end-to-end evidence for one complete password+MFA flow, each social provider, and each
  supported passwordless channel.
- Record security review of recovery, callback, linking, enumeration, and abuse controls.
