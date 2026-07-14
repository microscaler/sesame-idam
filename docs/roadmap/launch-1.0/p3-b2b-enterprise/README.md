# P3 — B2B/RBAC and Enterprise Wedge

**Target:** Launch 1.1

**Outcome:** organizations can manage fine-grained authorization, machine credentials,
event delivery, enterprise sign-on, and directory provisioning without operator intervention.

## Scope and dependencies

P3 depends on stable P0 tokens, P2 account lifecycle, and delivered organization membership.
It includes role-to-permission resolution, API-key lifecycle, webhooks, per-org SSO, and SCIM.
Arbitrary policy-language design and every identity provider/directory vendor are out of scope.

## Functional requirements

| ID | Requirement |
|---|---|
| FR-P3-001 | Effective principal resolution MUST derive roles and permissions from tenant-scoped assignments and include bounded, versioned claims in issued tokens. |
| FR-P3-002 | `POST /authorize` MUST evaluate principal, organization, resource/action, token version, and applicable policy for routes requiring online decisions. |
| FR-P3-003 | Personal and organization API keys MUST support create, one-time secret display, hashed storage, scope, expiry, rotate, revoke/archive, and last-used metadata. |
| FR-P3-004 | Webhooks MUST support tenant-scoped subscriptions, stable event IDs, HMAC signatures, secret rotation, bounded retry, delivery history, and replay. |
| FR-P3-005 | Organization admins MUST configure and validate SAML or OIDC SSO, domain/routing rules, certificate/secret rotation, and break-glass recovery. |
| FR-P3-006 | SCIM MUST implement the supported User/Group discovery, create, read, update, deactivate, filter, and pagination profile with idempotent external IDs. |
| FR-P3-007 | Permission, key, webhook, SSO, and SCIM administrative changes MUST create attributable audit events and invalidate affected authorization state. |

## Non-functional requirements

| ID | Requirement |
|---|---|
| NFR-P3-001 | Permission and organization boundaries MUST be enforced at storage and service layers with cross-tenant and privilege-escalation tests. |
| NFR-P3-002 | API-key and SSO/webhook secrets MUST be encrypted or one-way hashed, access-controlled, rotatable, and absent from observability data. |
| NFR-P3-003 | Webhook delivery MUST be at-least-once with documented ordering limits, idempotency guidance, exponential backoff, and a dead-letter/operator recovery path. |
| NFR-P3-004 | Online authorization, SCIM bulk, and webhook dispatch MUST each have capacity, latency, and back-pressure budgets before acceptance. |
| NFR-P3-005 | SAML/OIDC/SCIM support MUST publish tested protocol profiles and vendor compatibility rather than claim unrestricted conformance. |

## Acceptance criteria

| ID | Observable evidence |
|---|---|
| AC-P3-001 | Changing a role changes effective permissions and invalidates stale authorization; cross-org assignments never appear in claims or decisions. |
| AC-P3-002 | A key can be created, used within scope, denied outside scope, rotated without exposing stored plaintext, and immediately rejected after revoke/archive. |
| AC-P3-003 | A signed webhook is delivered, retried after transient failure, visible in delivery history, replayable, and verifiable during signing-secret rotation. |
| AC-P3-004 | An org admin self-configures one supported SAML and one supported OIDC integration, including certificate/secret rotation and break-glass recovery. |
| AC-P3-005 | A supported SCIM client provisions, updates, groups, filters, paginates, and deactivates users idempotently without affecting another tenant. |
| AC-P3-006 | Privilege-escalation, confused-deputy, replay, key-leak, signature-tamper, and cross-tenant suites pass. |

## Exit evidence

- Publish protocol/vendor matrices, authorization claim-size limits, API-key policy, webhook
  delivery contract, and SSO break-glass runbook.
- Link load/capacity results and tenant-isolation/security review for every sub-capability.
