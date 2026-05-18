---
title: Email Verification Entity
status: verified
updated: 2026-05-17
sources: [microservices/idam/identity-user-mgmt-service/impl/src/models/email_verification.rs]
---

# Entity: Email Verification

Owned by: **identity-user-mgmt-service**

## Description

Stores email verification tokens issued during user registration and email changes. Each record holds a signed token and its expiration timestamp. When a user clicks the verification link, the token is validated and the corresponding `users.email_verified` flag is set to `true`. The `ON DELETE CASCADE` on `user_id` ensures orphaned verification records are cleaned up when a user is deleted.

## Schema (from impl/ crate — identity-user-mgmt-service)

| Column | Type | Notes |
|--------|------|-------|
| id | uuid (PK) | |
| user_id | uuid (FK) | FK → `sesame_idam.users(id) ON DELETE CASCADE` |
| token | varchar(64) | Verification token string |
| expires_at | timestamptz | Token expiration |
| created_at | timestamptz | Record creation time |
| updated_at | timestamptz | Last update time |

## Key Design Decisions

1. **Token size limited to 64 chars.** The `token` column is `varchar(64)`, constraining the maximum token length. Tokens are likely HMAC or random string based, not JWTs.
2. **Cascade delete on user deletion.** When a user is deleted, all associated email verification records are automatically removed via `ON DELETE CASCADE` on `user_id`.
3. **Single active token per user at a time (implied).** The model does not include a `used` flag or `is_valid` column — tokens are presumably invalidated by expiration (`expires_at`) rather than an explicit consumed state.
4. **Always has updated_at.** Even though no update mechanism is documented, the column exists for consistency with other models.

## Code Anchors

- `microservices/idam/identity-user-mgmt-service/impl/src/models/email_verification.rs` — Lifeguard entity definition
- `microservices/idam/identity-user-mgmt-service/impl/src/controllers/verify_user_email.rs` — Verification endpoint handler
- `microservices/idam/identity-user-mgmt-service/impl/src/controllers/resend_email_confirmation.rs` — Resend token handler

## Gaps / Drift

> None — this entity was just created and verified against the impl model on 2026-05-17.
