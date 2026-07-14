# Launch 1.0 — Expanded Roadmap

This specification set turns the strategic [Launch 1.0 roadmap](../../ROADMAP-launch-1.0.md)
into testable delivery contracts. It adds scope boundaries, dependencies, functional and
non-functional requirements, acceptance criteria, and evidence required to exit each phase.

## Roadmap evaluation

The roadmap has a strong differentiator and a sensible dependency order: harden the token
boundary, prove the RLS moat, complete the customer-facing authentication surface, then make
the integration easy. Three issues previously made it difficult to execute:

1. **A delivery milestone was described as a product launch.** The dated
   [Hauliage delivery roadmap](../../audit/delivery-roadmap-2026-07-13.md) describes a six-week
   D3/D4 cut whose purpose is to enable initial Hauliage test users. It is now explicitly a
   **test-user enablement milestone**, while the strategic P0+P1+P2+P4 scope remains
   **Launch 1.0 General Availability (GA)**.
2. **Feature bullets lacked completion contracts.** A controller existing was easy to confuse
   with a secure, operable capability. Each phase now has requirement IDs and an evidence gate.
3. **Cross-cutting quality was aspirational.** Security, tenancy, observability, compatibility,
   documentation, and performance are now part of every phase's Definition of Done.

The estimates remain planning ranges, not commitments. A phase is complete only when its exit
gate is evidenced; elapsed time or endpoint count alone cannot change its status.

## Current delivery assessment (2026-07-13)

This is a repository-evidence assessment, not a percentage-complete estimate. A phase remains
unaccepted until every MUST requirement has evidence.

| Milestone/phase | Assessment | Evidence and principal gap |
|---|---|---|
| Hauliage test-user enablement | `in-progress` | The dated audit records the frozen D3/D4 consumer contract as functionally complete. Remaining cross-repo/environment acceptance and minimum revocation evidence must be closed before onboarding test users. |
| P0 | `in-progress` | Ed25519/JWKS, `typ`/algorithm validation foundations and denylist writes exist. The Sesame denylist middleware still contains a placeholder Redis lookup and is not wired in service `main.rs`; cross-consumer read-side/version evidence is therefore incomplete. |
| P1 | `not-started` | RLS and `SesameExecutor` appear in design/README material, but no non-document implementation anchors were found. |
| P2 | `in-progress` | Password register/login is delivered. The user-management/MFA/verification/social/passwordless surface still contains generated stubs or TODO-only handlers and does not meet the phase journey gate. |
| P3 | `in-progress` | Organization lifecycle and API-key validation foundations exist. Permissions-in-token, complete key lifecycle, webhook delivery, SSO and SCIM do not meet the phase gate. |
| P4 | `not-started` | No SDK/client directory or hosted-UI implementation was found; the developer contract remains design material. |
| P5 | `in-progress` | Audit/revocation/observability foundations exist, but the trust-and-scale control set and evidence gate remain future work. |

The assessment SHOULD be updated only with links to accepted commits and evidence. Stub files,
OpenAPI surface, design text, and passing compilation alone do not advance a phase to `accepted`.

## Release model

| Release | Scope | Purpose | Exit authority |
|---|---|---|---|
| **Hauliage test-user enablement** | D3/D4 consumer contract plus the minimum security, environment, and cross-repo work needed for initial test users | Onboard test users and prove the delivered identity/organization flow in Hauliage; not a Sesame product release | [Enablement specification](./hauliage-test-user-enablement/README.md), informed by the [dated audit](../../audit/delivery-roadmap-2026-07-13.md) |
| **1.0 GA** | P0 + P1 + P2 + P4 | Ship the product wedge: secure JWTs, RLS, credible auth, and excellent integration DX | This specification set |
| **1.1** | P3 | Add enterprise B2B/RBAC depth | [P3 specification](./p3-b2b-enterprise/README.md) |
| **1.2+** | P5 | Add trust, operational maturity, and advanced security | [P5 specification](./p5-trust-scale/README.md) |

The Hauliage milestone is a deployable test slice but intentionally consumer-specific. It MUST
NOT be marketed as a Sesame release or as satisfying the GA product promise. GA may reuse its
evidence where the implementation and acceptance criterion are identical and still current.

## Phase specifications

| Phase | Outcome | GA status | Detail |
|---|---|---|---|
| Enablement | Initial Hauliage test users can complete the supported identity and organization journeys | Pre-GA milestone | [Hauliage test-user enablement](./hauliage-test-user-enablement/README.md) |
| P0 | Every accepted token is standards-hardened and revocation-aware | Required | [Harden the core](./p0-harden-core/README.md) |
| P1 | Validated identity context enforces tenant isolation in PostgreSQL RLS | Required | [RLS bridge](./p1-rls-bridge/README.md) |
| P2 | A user can complete the principal authentication and account-lifecycle flows | Required | [Auth surface](./p2-auth-surface/README.md) |
| P3 | Organizations gain permissions, production API keys, webhooks, SSO, and SCIM | 1.1 | [B2B enterprise](./p3-b2b-enterprise/README.md) |
| P4 | A new application can adopt login, orgs, and RLS in less than one working day | Required | [Developer contract](./p4-developer-contract/README.md) |
| P5 | Operators gain advanced defense, evidence, and scale controls | 1.2+ | [Trust and scale](./p5-trust-scale/README.md) |

## Requirement language and traceability

- **MUST** is required to exit the phase; **SHOULD** is expected unless an ADR records why it
  is deferred; **MAY** is optional.
- `FR-Px-nnn` identifies functional behavior, `NFR-Px-nnn` a quality attribute, and
  `AC-Px-nnn` observable acceptance evidence.
- Every implementation PR MUST name the requirement IDs it satisfies and the tests or other
  evidence that verify them.
- Status values are `not-started`, `in-progress`, `blocked`, `accepted`, or `deferred`.
  Only `accepted` satisfies a release gate.

## Global non-functional requirements

These apply to every phase in addition to its phase-specific requirements.

| ID | Requirement |
|---|---|
| NFR-G-001 | **Security:** trust-boundary failures MUST fail closed unless an ADR explicitly defines a bounded fail-open mode, its threat model, telemetry, and operator alert. |
| NFR-G-002 | **Tenancy:** every tenant-scoped operation MUST have same-tenant, cross-tenant, and existence-leakage tests. Missing or conflicting tenant context MUST be rejected. |
| NFR-G-003 | **Privacy:** logs, metrics, traces, and error bodies MUST NOT expose passwords, tokens, invitation capabilities, API-key secrets, OTPs, or unnecessary PII. |
| NFR-G-004 | **Compatibility:** public APIs MUST remain OpenAPI-conformant. Breaking changes require versioning and migration guidance; generated/user-owned boundaries MUST remain intact. |
| NFR-G-005 | **Reliability:** external dependencies MUST have explicit timeouts, bounded retries where safe, and documented degraded behavior. Retryable work MUST be idempotent. |
| NFR-G-006 | **Performance:** each phase MUST publish a representative baseline and regression budget before acceptance. CI or release evidence MUST show the budget is met. |
| NFR-G-007 | **Observability:** new critical paths MUST emit structured outcome/latency telemetry and actionable dependency-failure signals without high-cardinality secrets. |
| NFR-G-008 | **Deployability:** migrations and configuration changes MUST be repeatable on a clean environment, documented, and verified in the supported Kubernetes deployment path. |
| NFR-G-009 | **Accessibility:** user-facing UI delivered by Sesame MUST meet WCAG 2.2 AA for the supported flows and pass automated plus keyboard-only checks. |
| NFR-G-010 | **Quality:** `cargo check --workspace`, `just lint-rust`, unit tests, and relevant BDD/E2E suites MUST pass without retries masking failures. |

## Definition of Done for a phase

A phase is `accepted` only when all of the following are true:

- every MUST functional requirement and acceptance criterion is satisfied;
- phase and global non-functional requirements have recorded evidence;
- negative, abuse, tenant-isolation, and dependency-failure cases are covered;
- OpenAPI implementation markers and generated-code ownership remain truthful;
- operator, integration, migration, and rollback documentation is published;
- no unresolved Critical or High security defect affects the phase scope;
- the evidence links, owner, decision log, and accepted commit are recorded in the phase page.

## Dependency and sequencing model

P0 is the security prerequisite for safe Hauliage test-user enablement and GA. P1 and P2 can execute in parallel after their
P0 trust-boundary assumptions are stable. P4 may prototype early but cannot be accepted until
the P1 and P2 APIs it exposes are stable. P3 depends on the P0 token contract and P2 lifecycle;
P5 is continuous, with individual controls promoted when their own evidence gates pass.

## Decisions still requiring an ADR

- Exact availability and latency SLOs for GA after representative load baselines exist.
- Revocation dependency-outage policy: the current roadmap says fail-open; NFR-G-001 requires
  the risk, bounded duration, telemetry, and operator response to be made explicit.
- Supported PostgreSQL, browser, Node.js, SAML, and SCIM compatibility matrices.
- Audit-event retention and data-residency tiers.
