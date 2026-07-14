# P4 — The Developer Contract

**Target:** Launch 1.0 GA

**Outcome:** a new SaaS team can integrate login, organizations, protected routes, and RLS in
less than one working day using supported artifacts and documentation.

## Scope and dependencies

P4 depends on stable P0, P1, and P2 contracts. It includes a TypeScript frontend SDK, backend
admin client, hosted login/onboarding UI, BRRTRouter integration, quickstart, and sample app.
SDKs for additional languages and a general-purpose UI builder are out of scope for GA.

## Functional requirements

| ID | Requirement |
|---|---|
| FR-P4-001 | `@sesame-idam/frontend` MUST provide typed session/auth state, login/logout, token refresh, organization listing/switching, and accessible sign-in/org-switcher components. |
| FR-P4-002 | The backend admin client MUST provide typed access to the GA user and organization operations with pagination, errors, idempotency, and retries matching the API contract. |
| FR-P4-003 | Hosted login/onboarding MUST support the P2 GA flows, organization selection/creation, safe redirect validation, and documented theming. |
| FR-P4-004 | BRRTRouter MUST expose first-class Sesame authentication middleware and RLS executor integration with secure defaults and explicit override points. |
| FR-P4-005 | The quickstart MUST cover deployment/configuration, login, protected route, organization switch, RLS install/policy, local test, and production-readiness next steps. |
| FR-P4-006 | A version-pinned sample application MUST exercise the same commands and APIs as the quickstart in CI. |
| FR-P4-007 | SDK/API errors MUST expose stable machine-readable categories without leaking tokens, secrets, or internal provider details. |

## Non-functional requirements

| ID | Requirement |
|---|---|
| NFR-P4-001 | Supported browsers, Node.js/TypeScript versions, Sesame API versions, and BRRTRouter versions MUST be published and tested in CI. |
| NFR-P4-002 | Frontend artifacts MUST publish size and performance budgets; optional UI/components MUST be tree-shakeable where the toolchain supports it. |
| NFR-P4-003 | Hosted UI and components MUST meet WCAG 2.2 AA for supported flows and pass keyboard, screen-reader smoke, contrast, and automated checks. |
| NFR-P4-004 | SDK releases MUST use semantic versioning, reproducible builds, provenance/checksums, changelogs, and migration guidance for breaking changes. |
| NFR-P4-005 | Browser code MUST not require service credentials or persist long-lived secrets in insecure storage; redirect/origin/CSP guidance MUST be explicit. |
| NFR-P4-006 | Documentation snippets and the sample app MUST be executable in CI to prevent drift. |

## Acceptance criteria

| ID | Observable evidence |
|---|---|
| AC-P4-001 | A developer unfamiliar with Sesame completes deploy/configure, login, org switch, protected route, and RLS-isolated query in under eight working hours using only published docs. |
| AC-P4-002 | The frontend SDK refreshes an expiring session, propagates logout/revocation, restores auth state safely, and handles network/provider failure without exposing secrets. |
| AC-P4-003 | The hosted UI completes password, MFA, supported social, and organization onboarding flows with safe redirects and tenant branding. |
| AC-P4-004 | Compatibility CI passes across the published browser/Node/TypeScript/BRRTRouter matrix and the package-size budget is met. |
| AC-P4-005 | Accessibility evidence shows no Critical/Serious automated violations and successful keyboard plus screen-reader smoke journeys. |
| AC-P4-006 | A clean-environment sample-app CI run proves every quickstart command and expected result. |

## Exit evidence

- Publish packages, checksums/provenance, compatibility matrix, API reference, runnable sample,
  and production-hardening guide.
- Attach a timed usability study with at least three representative developers; record all
  blocking documentation defects and their resolution.
