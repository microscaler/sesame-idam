# Docs Catalog — Sesame-IDAM

Inventory of all design documents in `docs/` and their wiki merge status.

## Design Docs

| File | Wiki Pages | Merge Status |
|------|-----------|-------------|
| `design-doc.md` (1164 lines) | All entity pages, topics/ architecture, JWT, flows | Merged into wiki |
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
