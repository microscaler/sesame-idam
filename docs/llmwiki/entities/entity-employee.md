---
title: Employee Entity
status: verified
updated: 2026-05-17
sources: [microservices/idam/identity-user-mgmt-service/impl/src/models/employee.rs]
---

# Entity: Employee

Owned by: **identity-user-mgmt-service**

## Description

Stores organizational metadata for users within an enterprise context. Each employee record is linked to a `users` row and holds an internal employee ID, department, job title, and a nullable manager reference. The manager reference is a self-referencing FK to another user, enabling org-chart-style hierarchies. When the referenced manager is deleted, `manager_id` is set to `NULL` (not cascade-deleted), preserving the employee record.

## Schema (from impl/ crate — identity-user-mgmt-service)

| Column | Type | Notes |
|--------|------|-------|
| id | uuid (PK) | |
| user_id | uuid (FK) | FK → `sesame_idam.users(id) ON DELETE CASCADE` |
| employee_id | varchar(64) | Internal employee identifier |
| department | varchar(255) (nullable) | Department or team name |
| title | varchar(255) (nullable) | Job title |
| manager_id | uuid (FK) (nullable) | FK → `sesame_idam.users(id) ON DELETE SET NULL` |
| created_at | timestamptz | Record creation time |
| updated_at | timestamptz | Last update time |

## Key Design Decisions

1. **Manager set to NULL on delete, not cascade.** The `manager_id` FK uses `ON DELETE SET NULL`, so when a manager user is deleted, the employee record is preserved with a null manager. This is unlike `user_id` which cascades — employees are deleted if the user is deleted, but the manager relationship is gracefully lost.
2. **Employee ID is a string, not an integer.** The `employee_id` is `varchar(64)`, allowing alphanumeric identifiers (e.g., "EMP-00123") and supporting integration with existing HR systems that may use non-numeric IDs.
3. **Department and title are optional.** Both fields are nullable, allowing employee records without structured org data — useful for early-stage org setup or contractors without formal titles.
4. **Single-level hierarchy.** Only one `manager_id` field exists — there is no support for multiple managers, reporting chains, or org-unit hierarchy beyond a single parent reference.
5. **Not org-scoped.** Unlike org-mgmt entities, the employee model has no `org_id` FK — it is scoped purely by user identity and assumed to live within the user's tenant context.

## Code Anchors

- `microservices/idam/identity-user-mgmt-service/impl/src/models/employee.rs` — Lifeguard entity definition
- `microservices/idam/identity-user-mgmt-service/impl/src/controllers/fetch_employee.rs` — Fetch employee handler
- `microservices/idam/identity-user-mgmt-service/impl/src/models/mod.rs` — Module declaration (line 4)

## Gaps / Drift

> None — this entity was just created and verified against the impl model on 2026-05-17.
