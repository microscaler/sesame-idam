# LLM Wiki — Session Log

## [2026-05-17] Entity Wiki Pages — Comprehensive Audit and Fix

### Summary

Complete audit of all 17 entity wiki pages against the actual Lifeguard impl models in the `impl/` crates. Cross-referenced each page against every column in every impl model for matching services. Created 7 missing entity pages, fixed 1 existing page, verified 9 existing pages.

### Changes Made

**7 new entity pages created:**

| Entity | Service | Columns | Key Details |
|--------|---------|---------|-------------|
| entity-email-verification.md | identity-user-mgmt-service | 6 | FK cascade to users, token limited to 64 chars |
| entity-social-account.md | identity-user-mgmt-service | 8 | FK cascade to users, provider/user_id strings |
| entity-employee.md | identity-user-mgmt-service | 8 | Self-referencing manager_id (ON DELETE SET NULL) |
| entity-scim-user.md | org-mgmt | 7 | Minimal SCIM model, no FK to users table |
| entity-org-domain.md | org-mgmt | 6 | Domain verification status |
| entity-org-invite.md | org-mgmt | 8 | Timestamp-based acceptance (not boolean/status) |
| entity-org-membership.md | org-mgmt | 7 | FK cascade on org_id and user_id, role is free-form string |

**1 existing page corrected:**

| Entity | Issue Fixed |
|--------|-------------|
| entity-api-key.md | Added references to api_key_usage and archived_api_key impl models (endpoint, method, reason, archived_at columns) |

**10 existing pages verified as complete** — all impl columns present:
- entity-user.md, entity-session.md, entity-organization.md, entity-role.md, entity-permission.md, entity-application.md, entity-mfa-device.md, entity-audit-log.md, entity-webhook.md, entity-scim-user.md

**Index updated:**
- `docs/llmwiki/index.md` — All 17 entity pages listed with status `verified` (changed entity-webhook from `partially-verified` to `verified`)

### OpenAPI vs Impl Discrepancies (Documented in ERD)

The ERD documents 41 impl models across 6 services. 17 impl models have **no corresponding OpenAPI schema** — they are database-only entities queried via service APIs without dedicated REST endpoints. The ERD also documents 14 categories of schema mismatches where OpenAPI specs describe properties that don't exist in impl, or vice versa.

### Open Issues

| Entity | Issue |
|--------|-------|
| Role/Permission | OpenAPI spec says `application_id`, impl uses `org_id` — specs are stale |
| AuditEvent (all) | OpenAPI spec has 16 properties (event_action, hmac_signature, target_id, etc.) — doesn't match either impl version (8-col authz-core or 10-col user-mgmt) |
| Org | OpenAPI spec has 21 properties including slug, logo_url, domain_auto_join, SAML fields — impl has only 6 columns |
| Application | OpenAPI spec has `slug`, impl has OIDC fields (client_id, client_secret, redirect_uris) |
| ScimUser | OpenAPI spec uses SCIM protocol format (emails array, name object, roles) — impl is a simple 7-col table |
| WebhookSubscription | OpenAPI spec has 12 properties with delivery tracking — impl has 8 columns with `active` boolean, not `enabled` |

## [2026-05-17] Epics Location and Implementation Status

### Summary

Added epics documentation discoverability and implementation status tracking. Fresh agents were not finding `docs/Epics/` because it was never referenced in AGENTS.md or the wiki index.

### Changes Made

**AGENTS.md** — Added `docs/Epics/INDEX.md` to the docs catalog table with description. Added epics directory layout explanation below the table: `docs/Epics/{N}-{name}/stories/story-N.M.md` pattern, INDEX.md as canonical master index.

**INDEX.md** — Added `Status` column to the epic table. Added "Implementation Status" section with:
- Story-level status for all 9 epics (44 stories total)
- Epic 1 Story 1.1 marked as **Implementing** — detailed file inventory: `key_manager.rs` (807 lines, Ed25519 gen/sign/verify, KeyManager with rotation/revocation/health, 11 unit tests), `controllers/jwks.rs`, `controllers/admin_jwks_revoke.rs`, `jwks_client.rs`, `main.rs`
- All other 40 stories marked as **Design** — verified by searching impl/ for story keywords (`jwt_only`, `jwt_with_fallback`, `route_policy`, `RouteAuthCategory`, `RoutePolicyStore`, claims schema types, version cache, delegation `act` claim, caching, observability spans) — none found
- Updated overall status from "Design phase -- no code changes" to "Story 1.1 in implementation"

### Verification

Searched all impl/ crates for implementation keywords. Only Epic 1 (asymmetric JWT) has code. Confirmed via: `search_files` across all impl dirs for key terms returned matches only in `identity-session-service/impl/` for key_manager, jwks, Ed25519, KeyManager. Zero matches for route classification or claims schema code.

### Open Issues

- Story 1.1 is "implementing" but not yet verified as compiling. No check was run that the key_manager changes integrate cleanly with the rest of identity-session-service build.
- The INDEX.md status section will need updates whenever new stories move from design to implementing.

### Files Changed

| File | Action |
|------|--------|
| `docs/llmwiki/entities/entity-email-verification.md` | Created — verified against impl |
| `docs/llmwiki/entities/entity-social-account.md` | Created — verified against impl |
| `docs/llmwiki/entities/entity-employee.md` | Created — verified against impl |
| `docs/llmwiki/entities/entity-scim-user.md` | Created — verified against impl |
| `docs/llmwiki/entities/entity-org-domain.md` | Created — verified against impl |
| `docs/llmwiki/entities/entity-org-invite.md` | Created — verified against impl |
| `docs/llmwiki/entities/entity-org-membership.md` | Created — verified against impl |
| `docs/llmwiki/entities/entity-api-key.md` | Patched — added missing entity references |
| `docs/llmwiki/index.md` | Patched — added 7 new entities, fixed webhook status |
| `docs/llmwiki/topics/topic-entity-relationship-diagram.md` | Updated — comprehensive ERD + all 41 impl models + OpenAPI gaps |
| `docs/llmwiki/topics/topic-data-model.md` | Updated — full table list + 17 impl models without OpenAPI + 14 schema mismatches |
