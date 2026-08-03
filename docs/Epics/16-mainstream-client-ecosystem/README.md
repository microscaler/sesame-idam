# Epic 16: Mainstream Framework Client Ecosystem

> **Status:** In progress  
> **Program:** Standards-first OIDC provider  
> **Audit source:** [Non-BRRTRouter framework readiness audit](../../audit/non-brrtrouter-framework-readiness-2026-07-25.md)  
> **Dependencies:** Epics 11–15; versioned public provider profile and conformance fixtures  
> **Selection:** [client-ecosystem-selection-v1.md](../../standards-first-oidc/client-ecosystem-selection-v1.md) — **Rust only**; BFF-mandated; other languages deferred

## Outcome

Sesame’s Supported client investment is the Rust server library
(`sesame-idam-client`), exercised end-to-end by Hauliage as the first product
dogfood (email/password + Google via Sesame). Frontends always authenticate
through their own backend (BFF pattern).

Multi-language SDKs and Auth.js productization are explicitly deferred. The
Epic 15 portable contract remains the integration surface for any future
non-Rust BFF.

Client packages improve ergonomics; they do not compensate for provider
non-conformance.

## Guiding decision

Do not build a custom OAuth/OIDC stack in every language.

Each integration should use:

- the framework's maintained OIDC relying-party library;
- the framework's maintained JWT/JWKS resource-server validation;
- generated or hand-curated public tenant/admin API types;
- Sesame-specific presets, verified-principal mapping, organization helpers,
  and policy integration.

Where a framework already supports issuer-only OIDC cleanly, a tested reference
application may be more valuable and safer than a large SDK.

## Entry gate

Client implementation must not start until:

- Epic 12 Authorization Code + PKCE works with unmodified generic clients;
- Epic 13 exposes a coherent public issuer/auth/API surface;
- Epic 14 conformance and negative fixtures pass;
- Epic 15 publishes a versioned provider profile and public OpenAPI;
- client authentication, tenant derivation, refresh, and logout are stable;
- release ownership and maintenance capacity exist for the target ecosystem.

## Selection process

Framework/language selection is a product decision, not an assumption in this
epic. Score candidates on:

1. existing or committed customer demand;
2. size and relevance of the framework ecosystem;
3. quality of generic OIDC and JWT support;
4. gap between generic support and a good Sesame developer experience;
5. package publishing and security-maintenance capacity;
6. ability to run compatibility CI continuously;
7. availability of maintainers fluent in ecosystem conventions;
8. overlap with other adapters.

## Initial candidate set

The audit recommends evaluating:

| Candidate | Native foundation | Likely deliverable |
|---|---|---|
| Auth.js / TypeScript | Auth.js generic OIDC | Named provider plus Next.js, SvelteKit, and Express examples |
| ASP.NET Core / C# | `AddOpenIdConnect`, `AddJwtBearer` | Configuration extensions, claim mapper, API client, samples |
| Spring / Java | Spring Security OAuth2 Client and Resource Server | Starter/configuration properties, authority mapper, API client |
| Python | django-allauth and Authlib | Django preset, FastAPI/Authlib helpers, shared API client |
| Laravel / PHP | Socialite-compatible OIDC implementation | Named Socialite driver, verified token/profile mapping, API client |
| Rails / Ruby | OmniAuth OpenID Connect | Strategy/preset, Devise sample, API client |
| Go | `coreos/go-oidc`, `x/oauth2` | Small helper modules, middleware examples, API client |
| Rust outside BRRTRouter | Mature Rust OIDC/JWT libraries | Framework-neutral async client/adapters separate from may client |

This table is a candidate backlog, not a commitment to build all packages.

## Stories

| Story | Title | Active? | Result |
|---|---|---|---|
| 16.1 | Ecosystem selection and support tiers | **Yes** | Rust Supported + BFF mandate; others deferred (selection doc) |
| 16.2 | Shared SDK specification and release template | **Yes** (Rust-scoped) | Security, errors, conformance, provenance requirements for `sesame-idam-client` |
| 16.3 | Auth.js/TypeScript integration | Deferred | — |
| 16.4 | ASP.NET Core integration | Deferred | — |
| 16.5 | Spring/Java integration | Deferred | — |
| 16.6 | Python integrations | Deferred | — |
| 16.7 | Laravel/PHP integration | Deferred | — |
| 16.8 | Rails/Ruby integration | Deferred | — |
| 16.9 | Go integration | Deferred | — |
| 16.10 | Non-BRRTRouter Rust integration | Deferred | may client remains the Supported path |
| 16.11 | Compatibility CI | **Yes** (Rust-scoped) | Client contract sync + fixture matrix against provider |
| 16.12 | Hauliage external-contract dogfood | **Yes** | Hauliage BFF on public contract; password + Google via Sesame |

Active train: **16.1 → 16.2 → 16.12 → 16.11** (client hardening as needed for dogfood).

## Progress (2026-08-03)

- [x] 16.1 Selection locked (Rust-only, BFF mandate)
- [x] 16.12 Seed `hauliage` + Google OAuth metadata; Hauliage BFF public-edge config
- [x] 16.11 Rust client contract sync + public API base helper
- [ ] 16.12 Live Google credentials in cluster + E2E sign-in proof
- [ ] 16.2 Formalize Rust client release/provenance template

Evidence: [evidence/hauliage-google-dogfood-2026-08-03.md](./evidence/hauliage-google-dogfood-2026-08-03.md)

## Required package layers

For each selected ecosystem, decide explicitly which layers are justified.

### Layer A — OIDC provider preset

- issuer/client configuration;
- standard login callback wiring;
- profile and verified-principal mapping;
- refresh and provider logout integration;
- organization-switching workflow.

### Layer B — resource-server adapter

- framework-native JWKS validation configuration;
- strict issuer/audience/algorithm/type defaults;
- Sesame principal extraction;
- roles/permissions policy helpers;
- optional request/RLS context projection.

### Layer C — public API client

- typed tenant/user/org/invitation operations;
- user-token and service-token modes;
- structured errors;
- pagination, idempotency, safe retries, and rate-limit handling.

A selected ecosystem may need only one or two layers. Do not publish empty
wrappers where documentation and conformance-tested configuration are sufficient.

## Shared requirements for every selected client

- never decode-and-trust JWT payloads;
- use issuer discovery rather than hard-coded endpoint paths;
- validate issuer, audience, algorithm, token type, time, and nonce as applicable;
- use PKCE S256 and state;
- redact tokens, codes, verifiers, and secrets;
- use secure server-side refresh-token storage for web applications;
- expose optional no-organization state;
- map namespaced authorization claims only after validation;
- keep local app session logout distinct from provider session logout;
- distinguish user credentials from service/admin credentials;
- pass the shared positive and negative fixtures;
- publish supported provider/API/framework versions;
- use semantic versioning, changelogs, provenance, and vulnerability reporting.

## Rust-first rationale (selected)

Microscaler products (Hauliage first) already use a may-native Rust BFF/client
path. Investing in additional language SDKs before that path is dogfooded on the
public OIDC contract would split maintenance without a second customer.

Auth.js and other framework presets remain valid *future* candidates when a
Node (or other) BFF product appears with a named owner. Until then they are not
Epic 16 deliverables.

## Acceptance gate per client

- [ ] Clean application integrates using published package and documentation.
- [ ] Login uses issuer discovery and Authorization Code + PKCE.
- [ ] Framework-native ID-token validation succeeds.
- [ ] API access-token validation rejects the shared negative vectors.
- [ ] Refresh rotation and provider logout are demonstrated.
- [ ] Optional/active organization states map correctly.
- [ ] Roles and permissions integrate through native authorization hooks.
- [ ] Public API client uses stable errors, pagination, idempotency, and retries.
- [ ] Tokens and secrets are absent from logs and browser-insecure storage guidance.
- [ ] Compatibility CI records exact framework and provider profile versions.
- [ ] Package provenance, checksum/signing, changelog, and security policy exist.

## Cross-language compatibility CI

Every selected client runs against:

- the same provider image/version;
- the same registered public and confidential clients;
- the same discovery/JWKS/token fixtures;
- the same positive login, refresh, UserInfo, logout, and org-switch journeys;
- the same negative token and protocol corpus;
- key rotation with warm caches;
- provider outage and rate-limit responses;
- contract/API compatibility checks.

## Support tiers

Story 16.1 should assign one of:

- **Supported:** release-blocking CI, documented versions, security SLA, active maintainer;
- **Preview:** usable but no compatibility guarantee, explicit limitations;
- **Reference:** tested sample/configuration, no dedicated package;
- **Community:** discoverable but not represented as Sesame-supported.

The website and docs must not call a reference sample an SDK or a community
package officially supported.

## Exit condition

Epic 16 is complete for a selected ecosystem only when its integration uses the
same public provider and contract as every other ecosystem, passes the shared
conformance suite, and has an explicit maintenance owner. Provider-specific
workarounds are treated as provider defects to fix in Epics 11–15, not copied
into every client.
