# Epic 16: Mainstream Framework Client Ecosystem

> **Status:** Future — blocked by standards-first provider gates  
> **Program:** Standards-first OIDC provider  
> **Audit source:** [Non-BRRTRouter framework readiness audit](../../audit/non-brrtrouter-framework-readiness-2026-07-25.md)  
> **Dependencies:** Epics 11–15 complete; versioned public provider profile and conformance fixtures

## Outcome

Selected languages and frameworks have supported Sesame client libraries,
presets, or reference integrations that feel native to their ecosystems while
sharing one OIDC, token, claims, and public API contract.

This epic begins only after Sesame is independently consumable through generic
OIDC libraries. Client packages improve ergonomics; they do not compensate for
provider non-conformance.

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

| Story | Title | Result |
|---|---|---|
| 16.1 | Ecosystem selection and support tiers | Approved targets, package type, owners, versions, and maintenance policy |
| 16.2 | Shared SDK specification and release template | Common security, errors, telemetry, conformance, provenance, and docs requirements |
| 16.3 | Auth.js/TypeScript integration | Named provider and framework-native samples across supported Auth.js bindings |
| 16.4 | ASP.NET Core integration | OIDC login, JWT bearer, policy mapping, and typed public API client |
| 16.5 | Spring/Java integration | OAuth2 login, resource server, authority mapping, and typed public API client |
| 16.6 | Python integrations | django-allauth preset, FastAPI/Authlib helpers, and Python API client |
| 16.7 | Laravel/PHP integration | Socialite-compatible provider and PHP API client |
| 16.8 | Rails/Ruby integration | OmniAuth preset/strategy and Ruby API client |
| 16.9 | Go integration | OIDC/JWT helpers, middleware recipes, and Go API client |
| 16.10 | Non-BRRTRouter Rust integration | Async/framework-neutral Rust path without changing the may-native client |
| 16.11 | Cross-language compatibility CI | Every supported package runs the shared provider/fixture matrix |
| 16.12 | Hauliage external-contract dogfood | Existing product uses the same public contract and fixtures as customers |

Stories 16.3–16.10 are activated only for candidates selected by 16.1.

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

## Auth.js first-candidate rationale

If product selection confirms it, Auth.js is the strongest first adapter because
one provider definition can cover Next.js, SvelteKit, and Express while testing
the provider against a widely used generic OIDC stack.

The first adapter must remain thin:

- `type: "oidc"` with Sesame issuer;
- standard Auth.js PKCE/state/nonce checks;
- profile/claim mapping;
- JWT/session callback examples for access and refresh tokens;
- provider logout and organization-switch examples;
- no Sesame-specific replacement for Auth.js protocol handling.

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
