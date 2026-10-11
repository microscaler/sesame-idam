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
2. **State, not a flag.** A claim moves PENDING → VERIFIED / GRACE / REVOKED / EXPIRED / CONFLICT only through `sesame_common::domain_verify::lifecycle` (DNS TXT proof via `dns-probe`, time, or an admin withdrawing it); the old writable `verified` flag was dropped (story 17.3). History in `org_domain_events` (append-only).
3. **Auto-join.** `auto_join` can be set only on a VERIFIED claim (`update_org_domain`) and is cleared when the claim is revoked.
4. **Cascade delete on org deletion.** When an organization is deleted, all associated domain records are automatically removed.

## Code Anchors

- `microservices/idam/org-mgmt/impl/src/models/org_domain.rs` — Lifeguard entity definition
- `microservices/idam/org-mgmt/impl/src/services/domain_api.rs` — the organisation domain API (story 17.4); controllers `*_org_domain*.rs`

## Gaps / Drift

> Updated for stories 17.3 and 17.4 (2026-10-11).
