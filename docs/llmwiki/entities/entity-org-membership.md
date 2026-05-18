---
title: Org Membership Entity
status: verified
updated: 2026-05-17
sources: [microservices/idam/org-mgmt/impl/src/models/org_membership.rs]
---

# Entity: Org Membership

Owned by: **org-mgmt**

## Description

Represents a user's membership in an organization. This is the bridge table connecting `users` to `organizations` with role and status metadata. Each row means "this user is a member of this organization with this role." Membership status can be "pending" (awaiting acceptance of an invite) or "active" (confirmed member). This entity replaced the implicit org-user relationship in the original design — prior to its creation, org membership was not modeled as a dedicated table.

## Schema (from impl/ crate — org-mgmt)

| Column | Type | Notes |
|--------|------|-------|
| id | uuid (PK) | |
| org_id | uuid (FK) | FK → `sesame_idam.organizations(id) ON DELETE CASCADE` |
| user_id | uuid (FK) | FK → `sesame_idam.users(id) ON DELETE CASCADE` |
| role | varchar(255) | User's role within the org (e.g., "owner", "admin", "member") |
| status | varchar(32) | "pending" or "active" |
| created_at | timestamptz | Record creation time |
| updated_at | timestamptz | Last update time |

## Key Design Decisions

1. **Simple status: pending or active.** The `status` column (varchar(32)) tracks whether the membership has been confirmed. "pending" is set when an invite is created but not yet accepted; "active" is set upon acceptance.
2. **Cascade delete on both sides.** Both `org_id` and `user_id` use `ON DELETE CASCADE` — removing an org or a user automatically removes all their memberships.
3. **Role is a string, not a FK.** The `role` column is `varchar(255)`, not a FK to the `roles` table. This means roles are treated as free-form strings (e.g., "admin", "editor", "viewer") rather than predefined role objects. This is a notable simplification from designs that use a proper roles FK.
4. **No role inheritance or hierarchy.** Unlike the `roles` table (which also has no inheritance), memberships are flat — a user has a single role per org, not inherited from a parent role.
5. **One membership per user-org pair (implied).** There is no explicit unique constraint on `(org_id, user_id)` in the impl model, so duplicate memberships could theoretically exist. Uniqueness is enforced at the application layer.

## Code Anchors

- `microservices/idam/org-mgmt/impl/src/models/org_membership.rs` — Lifeguard entity definition
- `microservices/idam/org-mgmt/impl/src/controllers/add_user_to_org.rs` — Add user to org handler
- `microservices/idam/org-mgmt/impl/src/controllers/remove_user_from_org.rs` — Remove user from org handler
- `microservices/idam/org-mgmt/impl/src/controllers/fetch_users_in_org.rs` — Fetch users in org handler

## Gaps / Drift

> None — this entity was just created and verified against the impl model on 2026-05-17.
