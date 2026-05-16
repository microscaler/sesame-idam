---
title: Social Account Entity
status: verified
updated: 2026-05-17
sources: [microservices/idam/identity-user-mgmt-service/impl/src/models/social_account.rs]
---

# Entity: Social Account

Owned by: **identity-user-mgmt-service**

## Description

Links external OAuth providers to internal users. Each record represents one social account (e.g., Google, GitHub) attached to a user. The `provider` column identifies the OAuth provider, while `provider_user_id` stores the provider's opaque user identifier. Access and refresh tokens are stored nullable — not all providers return a refresh token, and some flows (like implicit) don't issue them at all. This model is separate from `social_credentials` (tracked by identity-login-service) — identity-user-mgmt-service uses it for account linking via `link_social_account`.

## Schema (from impl/ crate — identity-user-mgmt-service)

| Column | Type | Notes |
|--------|------|-------|
| id | uuid (PK) | |
| user_id | uuid (FK) | FK → `sesame_idam.users(id) ON DELETE CASCADE` |
| provider | varchar(64) | OAuth provider name (e.g., "google", "github") |
| provider_user_id | varchar(255) | Opaque user ID from the provider |
| access_token | text (nullable) | OAuth access token |
| refresh_token | text (nullable) | OAuth refresh token |
| created_at | timestamptz | Record creation time |
| updated_at | timestamptz | Last update time |

## Key Design Decisions

1. **Nullable tokens.** Both `access_token` and `refresh_token` are `text (nullable)` — social accounts can exist without stored tokens (e.g., after token revocation or for providers that don't issue refresh tokens).
2. **Provider identified by name.** The `provider` column is a varchar(64) string, not a FK to a providers table. Provider definitions are handled externally (in the OpenAPI spec or config).
3. **Cascade delete on user deletion.** When a user is deleted, all linked social accounts are automatically removed via `ON DELETE CASCADE`.
4. **Unique linking enforced externally.** There is no unique constraint on `(provider, provider_user_id)` in the impl model, so uniqueness must be enforced at the application layer.
5. **Separate from social_credentials.** The identity-login-service has a `social_credentials` entity; this model in identity-user-mgmt-service serves account linking via `link_social_account` controller.

## Code Anchors

- `microservices/idam/identity-user-mgmt-service/impl/src/models/social_account.rs` — Lifeguard entity definition
- `microservices/idam/identity-user-mgmt-service/impl/src/controllers/link_social_account.rs` — Link social account handler
- `microservices/idam/identity-user-mgmt-service/impl/src/controllers/fetch_user_oauth_tokens.rs` — Fetch stored tokens
- `microservices/idam/identity-user-mgmt-service/impl/src/controllers/fetch_fresh_oauth_token.rs` — Refresh OAuth token
- `microservices/idam/identity-user-mgmt-service/impl/src/controllers/oauth_logout.rs` — Logout from OAuth provider

## Gaps / Drift

> None — this entity was just created and verified against the impl model on 2026-05-17.
