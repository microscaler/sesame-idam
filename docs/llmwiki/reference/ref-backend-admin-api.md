---
title: Backend Admin API
status: partially-verified
updated: 2026-01-22
sources: [sesame-idam-complete.md]
---

# Backend Admin API

The developer uses the Backend Admin API to manage users and orgs server-side.

## API Surface

```typescript
// Org management
const orgs = await sesame.orgs.list('user_abc123');
const org = await sesame.orgs.get('org_xyz789');
await sesame.orgs.update('org_xyz789', { name: 'Acme Corp 2.0' });
await sesame.orgs.close('org_xyz789');
const members = await sesame.orgs.getMembers('org_xyz789');
await sesame.orgs.addMember('org_xyz789', { userId: 'user_abc123', role: 'Admin' });
await sesame.orgs.removeMember('org_xyz789', 'user_abc123');

// User management
const users = await sesame.users.list({ limit: 10 });
const user = await sesame.users.get('user_abc123');
await sesame.users.update('user_abc123', { metadata: { tier: 'enterprise' } });
await sesame.users.delete('user_abc123');  // Soft delete

// Permission checks
const hasPermission = await sesame.permissions.check({
  userId: 'user_abc123',
  orgId: 'org_xyz789',
  permission: 'invoices:write'
});

// Impersonation
const impersonationToken = await sesame.users.impersonate('user_abc123');
```

## Code Anchors

- `openapi/*/openapi.yaml` — API spec for each service
- `microservices/idam/*/impl/src/` — Backend handler implementations

## Gaps / Drift

> **Open:** Verify actual admin API endpoints against the documented contract.
