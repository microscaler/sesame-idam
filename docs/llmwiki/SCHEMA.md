# SCHEMA.md — Sesame-IDAM llmwiki Conventions

## Purpose

This file defines the structure, conventions, and workflow for the Sesame-IDAM wiki. Every page in this wiki follows these rules.

## Source of Truth Order

1. **Runtime code** (microservices/*/impl/src/)
2. **Generated code** (microservices/*/gen/)
3. **OpenAPI specs** (openapi/*/openapi.yaml)
4. **Design docs** (docs/design-doc.md, docs/service-topology-design.md, docs/sesame-idam-complete.md)
5. **This wiki** (synthesis of all above — not a primary source)

## Page Format

Every wiki page follows this structure:

```markdown
---
title: Page Title
status: verified|partially-verified|unverified
updated: YYYY-MM-DD
sources: [source1.md, source2.md]
---

# Page Title

Page content...

## Status: verified

> This page has been checked against source code. Last verified: YYYY-MM-DD.

## Code Anchors

Point to actual files and line numbers where possible:

- `microservices/idam/identity-login-service/impl/src/models/user.rs:42`
- `openapi/identity-login-service/openapi.yaml:/paths`

## Gaps / Drift

List any differences between this page and the current codebase:

> **Open:** This page describes the User entity as it exists in the design doc. The actual impl models in the codebase may have diverged. Need to verify.
```

## Status Tags

| Tag | Meaning |
|-----|---------|
| `verified` | Checked against source code within last 7 days |
| `partially-verified` | Checked partially, or mixed results (some pages verified, some not) |
| `unverified` | Conceptual page, no implementation checked yet |

## Directory Layout

```
llmwiki/
├── SCHEMA.md              # This file — conventions only
├── index.md               # Content catalog by category
├── log.md                 # Append-only chronological session log
├── docs-catalog.md        # Inventory of docs/ files with merge status
├── entities/              # Key data structures (User, Org, etc.)
│   ├── entity-user.md
│   ├── entity-organization.md
│   └── ...
├── topics/                # Architectural concepts, workflows, standards
│   ├── topic-architecture-overview.md
│   ├── topic-jwt-schema.md
│   └── ...
└── reference/             # API surfaces, external integrations
    ├── ref-api-surface.md
    └── ...
```

## Agent Workflow

When making changes:

1. **Before writing**: Check if the page already exists. Update it rather than creating duplicates.
2. **After writing**: Update `index.md` if adding a new category. Append a `log.md` entry.
3. **After testing/verification**: Update the `status` tag.
4. **If code has diverged**: Note it in the "Gaps/Drift" section. Fix the code anchor if wrong.

## Cross-References

Use relative links from wiki root:

```markdown
See [entity-user](./entities/entity-user.md) for user data structure.
See [topic-architecture](./topics/topic-architecture-overview.md) for service overview.
```

Do NOT use absolute paths or external links in cross-references (they break when the wiki is read in different contexts).
