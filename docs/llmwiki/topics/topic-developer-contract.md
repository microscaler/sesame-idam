---
title: Developer Contract
status: partially-verified
updated: 2026-01-22
sources: [sesame-idam-complete.md]
---

# Developer Contract

The developer contract defines what a consuming application interacts with when integrating Sesame-IDAM.

## Three Integration Layers

```
┌─────────────────────────────────────────────┐
│           Consuming Application             │
│  ┌────────────────┬────────────────────┐   │
│  │ Frontend SDK   │ Backend Admin API  │   │
│  │ (user-facing)  │ (server-facing)    │   │
│  └────────┬───────┴────────┬───────────┘   │
├───────────┼────────────────┼────────────────┤
│           │ Sesame-IDAM    │                │
│  ┌────────▼────────────────▼───────────┐   │
│  │ RLS Helper SQL │ JWT │ Auth API │ Webhooks │   │
│  └──────────────────────────────────────────┘   │
└─────────────────────────────────────────────────┘
```

### 1. Frontend SDK (User-Facing)

```typescript
import { useAuth } from '@sesame-idam/frontend';

const { user, orgs, isLoading, login, logout } = useAuth();
// login({email}) — Handles the email/password (or magic link) flow
// useAuth() — Returns the current user's Enriched JWT
// user.switchOrg(id) — Rotates the JWT for new org context
```

### 2. Backend Admin API (Server-Facing)

```typescript
const orgs = await sesame.orgs.list('user_abc123');
const org = await sesame.orgs.get('org_xyz789');
const users = await sesame.users.list({ limit: 10 });
const hasPermission = await sesame.permissions.check({
  userId: 'user_abc123',
  orgId: 'org_xyz789',
  permission: 'invoices:write'
});
```

### 3. RLS Helper SQL (Database-Level)

```sql
-- Inject session context
SELECT sesame_set_session('validated-jwt-payload');

-- Query current context
SELECT sesame_current_user_id();
SELECT sesame_current_org_id();
```

## Code Anchors

- `docs/sesame-idam-complete.md:365-486` — Full developer contract section
- `clients/` — TypeScript SDK (if exists)

## Gaps / Drift

> **Open:** Verify SDK availability and API surface against implementation.
