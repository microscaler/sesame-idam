---
title: Org Invite Entity
status: verified
updated: 2026-05-17
sources: [microservices/idam/org-mgmt/impl/src/models/org_invite.rs]
---

# Entity: Org Invite

Owned by: **org-mgmt**

## Description

Stores pending invitations to join an organization. When an org admin invites a user by email, a record is created with a unique token and an expiration timestamp. The invited user clicks a link containing the token to accept the invite, which transitions them into an `org_memberships` record with the specified role. Invites can be revoked by the org admin before acceptance.

## Schema (from impl/ crate — org-mgmt)

| Column | Type | Notes |
|--------|------|-------|
| id | uuid (PK) | |
| org_id | uuid (FK) | FK → `sesame_idam.organizations(id) ON DELETE CASCADE` |
| email | varchar(255) | Invite recipient email |
| role | varchar(255) | Role assigned upon acceptance |
| token | varchar(64) | Unique invite token (for acceptance link) |
| expires_at | timestamptz | Invite expiration time |
| created_at | timestamptz | Record creation time |
| accepted_at | timestamptz | When the invite was accepted (nullable) |

## Key Design Decisions

1. **Acceptance recorded, not state change.** The `accepted_at` column is a timestamp, not a boolean. There is no `status` column (e.g., "pending", "accepted", "expired"). Invites are presumably considered "active" if `accepted_at IS NULL AND expires_at > now()`. The actual revocation (deletion) of invites is handled via the `revoke_pending_invite` controller.
2. **Token-based acceptance.** The `token` column (varchar(64)) contains the invite token embedded in the acceptance link. This avoids exposing the invite record's primary key in URLs.
3. **Role assigned at invite time.** The `role` column captures the role at invitation time — it is not dynamically looked up from the roles table. This simplifies the acceptance flow but means role changes after invitation require re-invitation.
4. **Email-only, not user-linked.** Invites are sent to email addresses — they do not reference existing users. If the invitee already has a user account, the acceptance flow would need to link the org_membership to the existing user.
5. **Cascade delete on org deletion.** When an organization is deleted, all pending invites are automatically removed.

## Code Anchors

- `microservices/idam/org-mgmt/impl/src/models/org_invite.rs` — Lifeguard entity definition
- `microservices/idam/org-mgmt/impl/src/controllers/invite_user_to_org.rs` — Create invite handler
- `microservices/idam/org-mgmt/impl/src/controllers/revoke_pending_invite.rs` — Revoke invite handler
- `openapi/org-mgmt/openapi.yaml` — Invite API endpoints

## Gaps / Drift

> None — this entity was just created and verified against the impl model on 2026-05-17.
