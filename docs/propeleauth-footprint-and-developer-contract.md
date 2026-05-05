# Sesame-IDAM Developer Contract & PropelAuth Footprint Analysis

> Purpose: Define the integration interface for Sesame-IDAM by reverse-engineering and expanding upon the PropelAuth API surface.
> Goal: Create a "Bolt-On" experience where a developer can implement complex B2B SaaS auth (users, orgs, roles) in <100 lines of code.

---

## 1. The PropelAuth API Footprint (The Benchmark)

PropelAuth works by providing a **Backend Admin API** (called by your server) and a **Frontend SDK** (called by the browser). The core contract is: **The developer never implements auth logic; they only orchestrate PropelAuth's entities.**

Here is the complete API surface we need to replicate (and improve upon):

### 1.1 User Management API
*Called by the Server to inspect or mutate the user base.*

| Action | HTTP Method | Endpoint | Description |
|--------|-------------|----------|-------------|
| **Get All Users** | `GET` | `/api/v1/users` | Returns paginated list of users. Supports filtering by email, creation date, and metadata. |
| **Get User By ID** | `GET` | `/api/v1/users/{userId}` | Returns full details of a specific user (email, metadata, createdAt, etc.). |
| **Update User** | `PUT` | `/api/v1/users/{userId}` | Updates user metadata or custom properties. Can also trigger password reset email. |
| **Delete/Deactivate User** | `DELETE` | `/api/v1/users/{userId}` | **Soft delete**. Deactivates the user immediately so they cannot log in, but preserves data. |
| **Impersonate User** | `POST` | `/api/v1/users/{userId}/impersonate` | Returns a temporary "Impersonation JWT" that grants the *caller* the exact permissions of the *target* user. |

### 1.2 Organization & Membership API
*The core B2B logic. Handling the "Company" concept.*

| Action | HTTP Method | Endpoint | Description |
|--------|-------------|----------|-------------|
| **List Orgs (for User)** | `GET` | `/api/v1/organizations` | Returns all organizations a specific user belongs to. |
| **Get Org Details** | `GET` | `/api/v1/organizations/{orgId}` | Returns org name, custom settings, allowed domains, and metadata. |
| **Update Org Settings** | `PUT` | `/api/v1/organizations/{orgId}` | Change org name, allowed email domains, or settings (e.g., enforce 2FA). |
| **Close Org** | `DELETE` | `/api/v1/organizations/{orgId}` | **Soft delete**. Deactivates the organization. Members become "unaffiliated". |
| **Get Org Members** | `GET` | `/api/v1/organizations/{orgId}/members` | Returns a list of all users in the org, their roles, and status. |
| **Change Member Role** | `PUT` | `/api/v1/organizations/{orgId}/members/{userId}` | **CRITICAL**. Promotes/Demotes a user (e.g., to "Admin"). Updates their effective permissions immediately. |
| **Remove Member** | `DELETE` | `/api/v1/organizations/{orgId}/members/{userId}` | **CRITICAL**. Removes a user from the org. They can still log in, but lose access to that org's data. |

### 1.3 Roles & Permissions API
*Defining what users can do.*

| Action | HTTP Method | Endpoint | Description |
|--------|-------------|----------|-------------|
| **Get Permissions** | `GET` | `/api/v1/permissions` | Lists all defined permissions and their descriptions. |
| **Get Roles** | `GET` | `/api/v1/roles` | Lists all roles (e.g., "Admin", "Editor") and their associated permissions. |
| **Check Permission** | `POST` | `/api/v1/permissions/check` | "Does user X in org Y have permission Z?" Returns true/false. |

### 1.4 Authentication (The JWT)
*PropelAuth's magic is the JWT.*

When a user logs in, the frontend SDK receives a JWT. This JWT contains:
*   `user_id`: The user's UUID.
*   `email`: Their email address.
*   `roles`: The roles assigned to the user *across all orgs*.
*   `permissions`: The flattened list of all permissions.
*   `custom_metadata`: Any custom properties set in the UI/API.

---

## 2. The Sesame-IDAM Developer Contract

This is the **Contract** we are building. When a developer integrates Sesame, this is exactly what they will interact with. We will match the PropelAuth API but add **Supabase-style RLS helpers** to give us a unique advantage.

### 2.1 Initialization (The "Hook")

The developer installs the SDK and initializes it with their **Platform API Key**.

```typescript
import { Sesame } from '@sesame-idam/sdk';

const sesame = new Sesame({
  // This key authenticates the SERVER to Sesame
  platformApiKey: 'sk_live_...' 
});
```

### 2.2 The Frontend SDK (The "Auth Layer")

The developer uses the frontend SDK to handle login. **They never write login logic.**

```typescript
// Frontend (React/Vue/HTML)
import { useAuth } from '@sesame-idam/frontend';

function App() {
  const { user, orgs, isLoading, login, logout } = useAuth();

  return (
    <div>
      {!user ? (
        <button onClick={() => login({ email: 'user@example.com' })}>
          Login
        </button>
      ) : (
        <div>
          <h1>Welcome, {user.email}</h1>
          <select onChange={(e) => user.switchOrg(e.target.value)}>
            {orgs.map(org => <option value={org.id}>{org.name}</option>)}
          </select>
        </div>
      )}
    </div>
  );
}
```

**The Contract:**
1.  `login({email})`: Handles the email/password (or magic link) flow.
2.  `useAuth()`: Returns the current user's **Enriched JWT**.
3.  `user.switchOrg(id)`: Rotates the JWT to reflect the new organization context.

### 2.3 The Backend API (The "Admin Layer")

The developer uses the Backend API to manage users and orgs server-side.

#### A. User Operations
```typescript
// Get a list of all users in the platform
const users = await sesame.users.list({ limit: 10 });

// Get a specific user by ID
const user = await sesame.users.get('user_abc123');

// Update user metadata (custom properties)
await sesame.users.update('user_abc123', {
  metadata: { tier: 'enterprise', seats: 50 }
});

// Deactivate a user (Soft Delete)
await sesame.users.delete('user_abc123');
```

#### B. Organization Operations
```typescript
// Get all orgs a user belongs to
const orgs = await sesame.orgs.list('user_abc123');

// Get specific org details
const org = await sesame.orgs.get('org_xyz789');

// Update org settings
await sesame.orgs.update('org_xyz789', {
  name: 'Acme Corp 2.0',
  settings: { allowed_email_domains: ['acme.com'] }
});

// Close an org
await sesame.orgs.close('org_xyz789');
```

#### C. Membership & Role Management
```typescript
// Get all members of an org
const members = await sesame.orgs.getMembers('org_xyz789');

// Add a user to an org (or update their role)
// This is the "Invite" flow logic
await sesame.orgs.addMember('org_xyz789', {
  userId: 'user_abc123',
  role: 'Admin' // or 'Editor'
});

// Remove a user from an org
await sesame.orgs.removeMember('org_xyz789', 'user_abc123');
```

#### D. Impersonation (The "Support" Feature)
```typescript
// Generate a token to act as the user
const impersonationToken = await sesame.users.impersonate('user_abc123');

// Use this token to make requests AS that user
const data = await api('/api/my-data', {
  headers: { Authorization: `Bearer ${impersonationToken}` }
});
```

### 2.4 The "RLS Bridge" (The Sesame Advantage)

This is where we beat the competition. PropelAuth gives you the JWT; Supabase gives you the RLS logic. Sesame gives you **both**.

We will provide a **PostgreSQL Helper Function** that the developer adds to their database once. It reads the JWT claims and enforces security at the database level.

**Developer Action:**
They run this SQL in their database (provided by Sesame CLI):

```sql
-- 1. Create the helper function
CREATE OR REPLACE FUNCTION public.current_user_org_id()
RETURNS UUID
LANGUAGE SQL
STABLE
AS $$
  SELECT nullif(current_setting('app.jwt.claims.org_id', true), '')::uuid;
$$;

-- 2. Enable RLS on your tables
ALTER TABLE my_custom_table ENABLE ROW LEVEL SECURITY;

-- 3. Create a policy that uses the function
CREATE POLICY "org_scoped_access" ON my_custom_table
  USING (org_id = public.current_user_org_id());
```

**Runtime:**
When the developer's server receives a request, they validate the Sesame JWT, and then pass the `org_id` to the DB session:

```typescript
// In your API handler
async function getTableData(req, res) {
  const jwt = await sesame.validateToken(req.headers.authorization);
  const orgId = jwt.claims.org_id; // e.g., "org_xyz789"
  
  // Set the context for Postgres
  db.query(`SET app.jwt.claims.org_id = '${orgId}'`);
  
  // Now run your query - the RLS policy automatically filters rows!
  const rows = await db.query('SELECT * FROM my_custom_table');
}
```

---

## 3. Summary of Differences & Strategy

| Feature | PropelAuth | Sesame-IDAM (New) |
| :--- | :--- | :--- |
| **User/Org Model** | Users + Orgs | Users + Orgs + **Tenants** (Optional 3rd layer) |
| **Database Security** | None (App logic only) | **Native RLS Helpers** (Database logic) |
| **UI** | Hosted Pages | **SDK Components** (React/Vue UI kit) |
| **Licensing** | Proprietary / Paid | **Open Source / Self-Hosted** |
| **Custom Metadata** | User/Org Metadata | User/Org Metadata + **Tenant Metadata** |

### Next Step
The contract is defined. The next step is to validate this structure against the **Sesame-IDAM Entity Model** we designed earlier to ensure every API endpoint has a corresponding database table and relationship.

Specifically, we need to map:
1.  `users` table -> `GET /api/v1/users`
2.  `organizations` table -> `GET /api/v1/organizations`
3.  `organization_member` table -> `PUT /api/v1/organizations/{id}/members`
4.  `jwt_claims` logic -> The RLS Bridge.

Shall we proceed to mapping the API endpoints to the Database Schema?