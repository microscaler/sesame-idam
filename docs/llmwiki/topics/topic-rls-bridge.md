---
title: RLS Bridge
status: partially-verified
updated: 2026-01-22
sources: [design-doc.md, docs/rls-design.md, docs/rls-design-v2.md, docs/rls-hauliage-design.md]
---

# RLS Bridge (Row-Level Security)

## Overview

Sesame provides SQL helper functions that inject RLS context into PostgreSQL sessions. The application never stores secrets in the database — no JWT ever enters the database.

## How It Works

1. Application middleware validates Sesame JWT (signatures, expiry)
2. Middleware calls `SET LOCAL` session variables to inject user/org context
3. Sesame-provided SQL helper functions set RLS policies
4. Application ORM queries are automatically scoped to the user's org

## Key Functions

- `sesame_set_session()` — Inject session context from validated JWT
- `sesame_current_user_id()` — Return current user from session
- `sesame_current_org_id()` — Return current org from session
- `sesame_current_user_role()` — Return user's role in current org

## Security Guarantees

1. **No JWT in DB.** The JWT never leaves the application. RLS context is set via `SET LOCAL` session variables.
2. **Database-level enforcement.** RLS policies prevent direct SQL queries from accessing data outside the current user's org.
3. **Session-scoped.** `SET LOCAL` only affects the current transaction — no cross-session leakage.

## Code Anchors

- `docs/rls-design.md` — Original RLS design
- `docs/rls-design-v2.md` — Updated RLS design
- `docs/rls-hauliage-design.md` — RLS design adapted from Hauliage

## Gaps / Drift

> **Open:** The actual RLS helper SQL is not yet in the repo. Verify when implemented.
