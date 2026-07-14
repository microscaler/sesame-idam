# P5 — Trust and Scale

**Target:** Launch 1.2+ as independently accepted increments

**Outcome:** operators can detect abuse, investigate and revoke sessions, produce trustworthy
audit evidence, and run Sesame under defined scale and compliance constraints.

## Scope and dependencies

P5 builds on all earlier phases. It covers audit experience/streaming, breach and bot defense,
DPoP/delegation/impersonation, device/session management, compliance evidence, and residency.
A certification claim is never implied by implementing controls alone; formal certification
and jurisdiction-specific legal advice remain separate programs.

## Functional requirements

| ID | Requirement |
|---|---|
| FR-P5-001 | Authorized operators MUST search, filter, inspect, export, stream, and retain audit events with tenant boundaries and evidence of actor, action, target, outcome, and time. |
| FR-P5-002 | Users/admins MUST list active devices/sessions, inspect meaningful metadata, and revoke one or all sessions with P0 enforcement. |
| FR-P5-003 | Password registration/reset MUST support breached-password screening with a privacy-preserving dependency contract and documented degraded mode. |
| FR-P5-004 | Authentication and recovery flows MUST detect/rate-limit brute force, credential stuffing, and anomalous automation and support operator review/override. |
| FR-P5-005 | DPoP-capable clients MUST bind tokens to approved keys and reject proof replay, method/URI mismatch, stale proof, and key mismatch. |
| FR-P5-006 | Delegation and impersonation MUST preserve actor/subject chains, constrain scopes/audience/time, require step-up/approval where configured, and be unmistakable in audit records. |
| FR-P5-007 | Compliance controls MUST map implementation, owner, test, evidence cadence, retention, and exceptions; evidence collection MUST be repeatable. |
| FR-P5-008 | Residency controls MUST constrain supported data classes and backups to configured regions and expose verifiable placement/transfer evidence. |

## Non-functional requirements

| ID | Requirement |
|---|---|
| NFR-P5-001 | Audit records MUST be append-only/tamper-evident at the defined trust boundary, time-synchronized, exportable, and governed by documented retention/deletion rules. |
| NFR-P5-002 | Abuse defenses MUST have measured false-positive/false-negative trade-offs, privacy review, accessible recovery, and safe operator override. |
| NFR-P5-003 | Session/revocation and audit pipelines MUST have availability, latency, throughput, backlog, and recovery SLOs backed by load and failure tests. |
| NFR-P5-004 | Advanced security algorithms/protocols MUST use maintained libraries, explicit allow-lists, rotation, and independent threat-model review. |
| NFR-P5-005 | Evidence and residency claims MUST be continuously testable and MUST distinguish implemented, operating, and independently certified states. |

## Acceptance criteria

| ID | Observable evidence |
|---|---|
| AC-P5-001 | An operator traces a complete security event, exports/verifies it, and proves another tenant cannot view or stream it. |
| AC-P5-002 | A user revokes one session and all sessions; affected tokens are rejected within the documented enforcement bound while unaffected sessions behave as specified. |
| AC-P5-003 | Breached-password and attack simulations trigger the expected controls without sending plaintext passwords or secrets to dependencies/logs. |
| AC-P5-004 | DPoP replay, altered method/URI, stale proof, and wrong-key tests fail; a conforming proof succeeds across the published client matrix. |
| AC-P5-005 | Delegated/impersonated actions enforce scope and expiry, require configured approval/step-up, and produce actor-plus-subject audit evidence. |
| AC-P5-006 | Disaster, dependency-outage, backlog, retention, evidence-collection, and residency-placement exercises meet their approved SLO/control targets. |

## Exit evidence

- Link threat models, load/failure reports, incident/operator runbooks, control matrix, and
  evidence samples with sensitive data removed.
- Record which controls are implemented, proven operating, independently assessed, or deferred;
  do not label the product certified without the applicable independent attestation.
