# Docs Catalog — Sesame-IDAM

Inventory of all design documents in `docs/` and their wiki merge status.

## Roadmaps

| File | Purpose | Status |
|---|---|---|
| `ROADMAP-launch-1.0.md` | Strategic product scope and phase index | Expanded and cross-linked |
| `roadmap/launch-1.0/README.md` | Roadmap evaluation, release model, global requirements, and phase index | Current |
| `roadmap/launch-1.0/hauliage-test-user-enablement/README.md` | “Just enough IDAM” requirements and evidence gate for initial Hauliage test users | Current |
| `roadmap/launch-1.0/p0-harden-core/README.md` | Token validation and revocation requirements | Current |
| `roadmap/launch-1.0/p1-rls-bridge/README.md` | RLS bridge requirements and zero-bleed gate | Current |
| `roadmap/launch-1.0/p2-auth-surface/README.md` | User lifecycle, MFA, verification, social, and passwordless requirements | Current |
| `roadmap/launch-1.0/p3-b2b-enterprise/README.md` | Permissions, API keys, webhooks, SSO, and SCIM requirements | Current |
| `roadmap/launch-1.0/p4-developer-contract/README.md` | SDK, hosted UI, BRRTRouter integration, and quickstart requirements | Current |
| `roadmap/launch-1.0/p5-trust-scale/README.md` | Audit, abuse defense, advanced security, and compliance requirements | Current |
| `audit/delivery-roadmap-2026-07-13.md` | Hauliage “just enough IDAM” test-user enablement record | Point-in-time audit |

## Design Docs

| File | Wiki Pages | Merge Status |
|------|-----------|-------------|
| `design-doc.md` (1164 lines) | All entity pages, topics/ architecture, JWT, flows | Merged into wiki |
| `design-saas-of-saas-multi-tenancy.md` | topic-platform-tenants.md, Epic 10 | **Canonical** — PRD/story source (2026-07-14) |
| `PRD-P1-platform-tenant-admin.md` | Epic 10 stories 10.1–10.8 | Draft — ready for implementation |
| `service-topology-design.md` (339 lines) | Architecture overview, scaling profiles, inter-service deps | Merged into wiki |
| `sesame-idam-complete.md` (1034 lines) | Vision, developer contract, API surface, benchmark | Partially merged |
| `adr-001-org-type-classification.md` | topics/topic-org-personas.md | Partially merged |
| `propelauth-gap-analysis.md` | reference/ref-propelauth-comparison.md | Referenced in wiki |
| `propelauth-api-footprint.md` | reference/ref-propelauth-comparison.md | Referenced in wiki |
| `propeleauth-footprint-and-developer-contract.md` | topics/topic-developer-contract.md | Partially merged |

## Design Diagrams

| File | Content |
|------|---------|
| `mermaid/HLD.md` | High-level architecture diagrams |
| `mermaid/sequence.md` | Sequence diagrams (login, authorize, key validation) |
| `mermaid/UML.md` | UML class diagrams |
| `mermaid/Complete DDL data source.md` | DDL/ERD data source |

## RLS Design Docs

| File | Wiki Pages | Merge Status |
|------|-----------|-------------|
| `rls-design.md` | topics/topic-rls-bridge.md | Partially merged |
| `rls-design-v2.md` | topics/topic-rls-bridge.md | Partially merged |
| `rls-hauliage-design.md` | topics/topic-rls-bridge.md | Referenced in wiki |

## Other Docs

| File | Notes |
|------|-------|
| `cross-repo-auth-analysis.md` | Cross-repo auth analysis — needs wiki page |
| `service-topology-design.md` | Already merged into wiki |

## Status Legend

| Status | Meaning |
|--------|---------|
| Merged | Content fully migrated to wiki |
| Partially merged | Some content migrated, rest needs review |
| Referenced in wiki | Cited as source in wiki pages |
| Needs merging | Design doc exists but not yet in wiki |
