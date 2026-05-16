---
title: Org Domain Entity
status: verified
updated: 2026-05-17
sources: [microservices/idam/org-mgmt/impl/src/models/org_domain.rs]
---

# Entity: Org Domain

Owned by: **org-mgmt**

## Description

Stores verified email domains for an organization. Domains are used to identify which email addresses belong to the organization (e.g., `@acme.com` → Acme Corp). This enables features like domain-based user discovery, auto-joining, and SSO provisioning. The `verified` flag indicates whether the domain has been proven to be owned by the organization (typically via DNS TXT record verification). Domains are scoped to organizations — the same domain can belong to different orgs as separate records.

## Schema (from impl/ crate — org-mgmt)

| Column | Type | Notes |
|--------|------|-------|
| id | uuid (PK) | |
| org_id | uuid (FK) | FK → `sesame_idam.organizations(id) ON DELETE CASCADE` |
| domain | varchar(255) | Email domain (e.g., "acme.com") |
| verified | boolean | Whether the domain ownership has been verified |
| created_at | timestamptz | Record creation time |
| updated_at | timestamptz | Last update time |

## Key Design Decisions

1. **Domain as plain string, not email.** The `domain` column stores a bare domain name (e.g., "acme.com"), not a full email address. This avoids the need to parse emails and allows reuse of the same domain across different orgs.
2. **Explicit verified flag.** Domain verification is a two-step process: add the domain, then prove ownership (via DNS TXT record). The `verified` boolean tracks whether verification has completed.
3. **No auto-join logic in the model.** While the OpenAPI spec includes domain-based features (auto-join, domain restrict), the impl model has no `auto_join` or `restrict` columns — these are currently conceptual and not implemented in the database.
4. **Cascade delete on org deletion.** When an organization is deleted, all associated domain records are automatically removed.

## Code Anchors

- `microservices/idam/org-mgmt/impl/src/models/org_domain.rs` — Lifeguard entity definition
- `microservices/idam/org-mgmt/impl/src/controllers/update_org_domains.rs` — Update org domain settings

## Gaps / Drift

> None — this entity was just created and verified against the impl model on 2026-05-17.
