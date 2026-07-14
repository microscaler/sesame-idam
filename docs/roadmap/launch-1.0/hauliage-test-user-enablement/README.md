# Hauliage Test-User Enablement

**Target:** approximately 2026-08-24, subject to evidence

**Outcome:** initial shipper and transporter test users can authenticate, establish an
organization context, and use the supported Hauliage journeys against the shared environment.
This is “just enough IDAM” for product testing, not a Sesame product release.

## Scope and boundaries

The source plan is the dated [D3/D4 delivery roadmap](../../../audit/delivery-roadmap-2026-07-13.md).
This milestone includes only the Sesame endpoints Hauliage calls, the role/persona claims those
journeys require, the minimum token lifecycle/security behavior safe for named test users, and
cross-repository environment/E2E proof.

SDKs, hosted UI, broad user administration, MFA/social/passwordless, enterprise SSO/SCIM,
general webhook delivery, full API-key lifecycle, and full endpoint parity are out of scope.
Deferral here does not remove them from the Launch 1.0 or later product roadmap.

The 2026-07-14 delivery decision adds a narrow RLS slice: one production-shaped Hauliage
organization-owned path must derive database context only from validated JWT claims and pass the
transaction/pool zero-bleed suite. General policy generation remains a Launch 1.0 deliverable.

## Functional requirements

| ID | Requirement |
|---|---|
| FR-HTE-001 | A permitted test user MUST register or use an approved seed account and authenticate with email/password through Hauliage's real Sesame integration. |
| FR-HTE-002 | Hauliage MUST obtain the current user and memberships, create or select an organization, and set the active organization using the frozen tenant-consumer contract. |
| FR-HTE-003 | The supported invitation journey MUST allow an authorized member to create an invitation and the intended recipient to preview and accept it safely. |
| FR-HTE-004 | Issued identity context MUST contain the organization, persona/role, issuer, audience, expiry, `jti`, and version data required by the Hauliage authorization path. |
| FR-HTE-005 | Shipper and transporter test personas MUST resolve to their approved roles/permissions; client-supplied role or tenant data MUST NOT override Sesame's authoritative state. |
| FR-HTE-006 | Refresh MUST rotate refresh tokens and reject reuse; logout MUST terminate refresh capability, denylist the current access token for its remaining lifetime, and make that access token fail on its next protected request. |
| FR-HTE-007 | Hauliage's BFF/frontend MUST use the shared-environment Sesame URLs and handle success, unauthenticated, forbidden, expired, and dependency-error responses without mock fallbacks. |
| FR-HTE-008 | The shared Kubernetes environment MUST expose the agreed service ports, database/Redis dependencies, migrations, seeds, and configuration required for repeatable test-user onboarding. |
| FR-HTE-009 | A protected Hauliage database path MUST execute through `SesameExecutor`, inject validated tenant/user/organization context with transaction-local semantics, and rely on PostgreSQL RLS rather than a client-supplied tenant predicate. |

## Non-functional requirements

| ID | Requirement |
|---|---|
| NFR-HTE-001 | Same-tenant and cross-tenant tests MUST prove no user can fetch, join, switch to, or administer an organization outside their authorized memberships. |
| NFR-HTE-002 | Test-user passwords, tokens, invitation capabilities, API keys, and unnecessary PII MUST be absent from logs, traces, screenshots, and committed fixtures. |
| NFR-HTE-003 | The shared environment MUST support a documented reset/reseed/retry procedure without manual database surgery for routine test runs. |
| NFR-HTE-004 | Authentication and organization journeys MUST emit enough structured telemetry to distinguish Sesame, Hauliage BFF, database, Redis, and configuration failures. |
| NFR-HTE-005 | The live E2E MUST pass from a clean browser/session with test retries disabled; a retry MUST NOT be used to classify an unstable journey as accepted. |
| NFR-HTE-006 | Dynamic token-status Redis failure MUST use the accepted fail-closed bounded policy in [ADR-003](../../../ADR-003-token-status-dependency-outage.md); any remaining security deferral MUST have an explicit test-user risk decision. |
| NFR-HTE-007 | RLS identity context MUST clear on commit, rollback, error, panic/cancellation, and pooled-connection reuse; absent or conflicting context MUST fail closed. |

## Acceptance criteria

| ID | Observable evidence |
|---|---|
| AC-HTE-001 | A new shipper test user registers, signs in, creates/selects the shipper organization, reaches the expected Hauliage dashboard, refreshes, signs out, and cannot reuse either the refresh token or the denylisted access token. |
| AC-HTE-002 | A transporter test user signs in, receives the transporter role/persona context, reaches only the transporter journey, and is denied shipper-only actions. |
| AC-HTE-003 | An authorized inviter creates an invitation; the intended user previews and accepts it, sees the membership, switches active organization, and receives updated authoritative claims. |
| AC-HTE-004 | Wrong-tenant headers, non-member organization IDs, invitation recipient mismatch, role spoofing, expired tokens, and refresh-token replay are rejected without existence or secret leakage. |
| AC-HTE-005 | The real-login Hauliage Playwright journey passes against the shared Kubernetes stack from a documented clean state with no Sesame or BFF mocks. |
| AC-HTE-006 | A fresh reset/reseed followed by the full test-user journey succeeds using only the published operator commands and configuration. |
| AC-HTE-007 | The owner records an explicit go/no-go decision for initial test users, including accepted commits, evidence links, known limitations, rollback, and support contact. |
| AC-HTE-008 | Two tenants with interleaved organization-owned rows repeatedly share the pool; unqualified reads return only the authenticated organization’s rows, while missing, forged-header, conflicting, rollback, and reuse cases return zero rows or an authorization error. |

## Exit evidence

- Record the exact Sesame, BRRTRouter, Hauliage, and environment commits/images under test.
- Attach the no-retry E2E output and focused Sesame unit/BDD quality-gate output.
- Publish the test-user onboarding/reset runbook, supported persona matrix, known limitations,
  and revocation-risk decision.
- Confirm that the milestone is labelled test-only and is not represented as Launch 1.0 GA.
