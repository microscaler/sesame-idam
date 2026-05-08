---
title: Frontend SDK
status: partially-verified
updated: 2026-01-22
sources: [sesame-idam-complete.md]
---

# Frontend SDK Integration

## The Contract

The developer uses the frontend SDK to handle authentication. They never write login logic.

## API

```typescript
import { useAuth } from '@sesame-idam/frontend';

function App() {
  const { user, orgs, isLoading, login, logout } = useAuth();

  return (
    <div>
      {!user ? (
        <button onClick={() => login({ email: 'user@example.com' })}>Login</button>
      ) : (
        <div>
          <h1>Welcome, {user.email}</h1>
          <select onChange={(e) => user.switchOrg(e.target.value)}>
            {orgs.map(org => <option value={org.id}>{org.name}</option>)}
          </select>
          <button onClick={logout}>Logout</button>
        </div>
      )}
    </div>
  );
}
```

## Methods

| Method | Description |
|--------|-------------|
| `login({email})` | Handles email/password (or magic link) flow |
| `useAuth()` | Returns current user's Enriched JWT |
| `user.switchOrg(id)` | Rotates JWT to reflect new org context |

## Code Anchors

- `ui/` — Frontend components (if exists)
- `clients/` — TypeScript SDK (if exists)

## Gaps / Drift

> **Open:** Verify actual SDK implementation vs the design contract.
