# Runbook: Google credentials for Sesame social login

Status: publishable  
Audience: platform operators and product teams enabling **Sign in with Google**  
Related: [ADR-004](../ADR-004-platform-tenant-provisioning.md), social login on identity-login-service

This runbook creates the Google OAuth **Web client** credentials that Sesame
loads into the identity-login-service process environment. It is
**tenant-scoped**: each platform tenant has its own Google app and env keys.

## How the flow works (read first)

Sesame does **not** receive the Google redirect. The product backend (BFF)
does.

```text
Browser
  → GET  {Sesame}/idam/v1/auth/social/google/login?redirect_uri={product_callback}
  → 302  Google authorize
Google
  → 302  {product_callback}?code=…&state=…
Product BFF
  → POST {Sesame}/idam/v1/auth/social/google/callback
       { "code", "state", "redirect_uri" }
  ← Sesame access_token / refresh_token
```

Therefore:

1. Google Cloud **Authorized redirect URIs** must list the **product**
   callback URL(s) (exact string match), not a Sesame hostname.
2. The same URI must appear in Sesame’s tenant OAuth allowlist
   (`tenant_oauth_providers.redirect_uris` and/or the matching env allowlist).
3. The browser never posts the Google `code` to Sesame; the BFF does.

This is separate from Sesame-as-OIDC-provider (`/oauth/authorize` + PKCE).
Social federation uses a confidential Google client + client secret.

## Naming convention

Tenant slug → env segment: uppercase, hyphens to underscores.

| Tenant slug | Env segment |
|---|---|
| `acme` | `ACME` |
| `hauliage` | `HAULIAGE` |
| `my-saas` | `MY_SAAS` |

Required keys for Google:

```text
SESAME_OAUTH__{TENANT}__GOOGLE_CLIENT_ID=<Google OAuth client id>
SESAME_OAUTH__{TENANT}__GOOGLE_CLIENT_SECRET=<Google OAuth client secret>
```

Optional (comma-separated exact redirect URIs; overrides/extends ops practice
when used by your deployment):

```text
SESAME_OAUTH__{TENANT}__ALLOWED_REDIRECT_URIS=<uri1>,<uri2>
```

Runtime resolution (identity-login-service):

- Secret is always read from the env var named in
  `tenant_oauth_providers.secret_env_key`.
- Client id is read from `client_id_env_key` when set and non-empty; otherwise
  from the DB `client_id` column (often a placeholder until env is populated).

Canonical key names for a tenant slug `T`:

```text
SESAME_OAUTH__{T_UPPER}__GOOGLE_CLIENT_ID
SESAME_OAUTH__{T_UPPER}__GOOGLE_CLIENT_SECRET
```

## Prerequisites

- Active platform tenant row (`tenants.slug`).
- Enabled `tenant_oauth_providers` row for `(tenant_slug, 'google')` with:
  - `secret_env_key` = `SESAME_OAUTH__{TENANT}__GOOGLE_CLIENT_SECRET`
  - `client_id_env_key` = `SESAME_OAUTH__{TENANT}__GOOGLE_CLIENT_ID` (recommended)
  - `redirect_uris` listing every product callback you will use
- Identity-login-service can reach `accounts.google.com` / `oauth2.googleapis.com`
- Redis available (OAuth `state` storage)

## Step 1 — Choose redirect URIs

List every origin where the product SPA (or BFF-hosted page) handles the Google
return. Typical patterns:

| Environment | Example callback |
|---|---|
| Local Vite / webpack | `http://localhost:7174/oauth/callback` |
| Local loopback | `http://127.0.0.1:7174/oauth/callback` |
| Dev ingress | `https://app.example.dev/oauth/callback` |
| Production | `https://app.example.com/oauth/callback` |

Rules:

- Scheme, host, port, and path must match **exactly** (trailing slash matters).
- Prefer HTTPS outside local development.
- Do **not** register Sesame’s `/idam/v1/auth/social/google/callback` — that
  route is a **JSON POST** API, not a browser redirect target.

## Step 2 — Create the Google Cloud OAuth client

1. Open [Google Cloud Console](https://console.cloud.google.com/) → select or
   create a project for this **product tenant** (one Google project per tenant
   is the usual isolation model).
2. **APIs & Services → OAuth consent screen**
   - User type: **External** (or Internal for Google Workspace–only).
   - App name, support email, and developer contact as required.
   - Scopes: at minimum `openid`, `email`, `profile` (Sesame requests these).
   - Add test users while the consent screen is in Testing.
3. **APIs & Services → Credentials → Create credentials → OAuth client ID**
   - Application type: **Web application**.
   - Name: e.g. `sesame-{tenant}-social` (label only).
   - **Authorized JavaScript origins** (if prompted): product origins only
     (e.g. `https://app.example.com`, `http://localhost:7174`).
   - **Authorized redirect URIs**: every URI from Step 1.
4. Copy the **Client ID** and **Client secret** immediately. Treat the secret
   as a credential: store in your secret manager, never commit to git.

## Step 3 — Register metadata in Sesame

Ensure the DB row exists and points at the env keys (seed, platform API, or CLI).

Illustrative values for tenant `{tenant}`:

| Column | Value |
|---|---|
| `tenant_slug` | `{tenant}` |
| `provider` | `google` |
| `client_id` | placeholder or the real client id |
| `client_id_env_key` | `SESAME_OAUTH__{TENANT}__GOOGLE_CLIENT_ID` |
| `secret_env_key` | `SESAME_OAUTH__{TENANT}__GOOGLE_CLIENT_SECRET` |
| `redirect_uris` | comma-separated exact callbacks from Step 1 |
| `enabled` | `true` |

Platform API shape (when available):

```http
PUT /idam/v1/platform/tenants/{tenant}/oauth/google
```

Body carries metadata only — **never** the client secret.

CLI shape (when available):

```bash
sesame-idam tenant oauth set \
  --slug "{tenant}" \
  --provider google \
  --client-id "$GOOGLE_CLIENT_ID" \
  --client-id-env-key "SESAME_OAUTH__{TENANT}__GOOGLE_CLIENT_ID" \
  --secret-env-key "SESAME_OAUTH__{TENANT}__GOOGLE_CLIENT_SECRET"
```

Update `redirect_uris` whenever you add an environment or change the callback path.

## Step 4 — Inject secrets into identity-login-service

Mount the two values into the identity-login-service process environment
(Kubernetes Secret, sealed-secrets/SOPS, cloud secret store → env, etc.).

Example Secret (illustrative):

```yaml
apiVersion: v1
kind: Secret
metadata:
  name: sesame-oauth-{tenant}-google
  namespace: sesame-idam
type: Opaque
stringData:
  SESAME_OAUTH__{TENANT}__GOOGLE_CLIENT_ID: "........apps.googleusercontent.com"
  SESAME_OAUTH__{TENANT}__GOOGLE_CLIENT_SECRET: "........"
```

Wire the Secret keys into the identity-login-service Deployment/Helm values as
environment variables with **those exact names**. Restart or roll the pods so
the new env is visible.

Do not place secrets in ConfigMaps, git, or OpenAPI examples.

## Step 5 — Verify

1. **Metadata present**

   ```bash
   # Platform get (when available), or SQL against tenant_oauth_providers
   # Expect enabled=true and the env key names above.
   ```

2. **Env present in the pod** (names only — do not print secret values)

   ```bash
   kubectl -n sesame-idam exec deploy/identity-login-service -- \
     sh -c 'test -n "$SESAME_OAUTH__{TENANT}__GOOGLE_CLIENT_ID" \
       && test -n "$SESAME_OAUTH__{TENANT}__GOOGLE_CLIENT_SECRET" \
       && echo ok'
   ```

3. **Start handshake** (expect `302` to `accounts.google.com`, not `503`)

   ```http
   GET /idam/v1/auth/social/google/login?redirect_uri={exact_product_callback}
   X-Tenant-ID: {tenant}
   ```

4. **Complete via product BFF** using the returned `code` and `state`, posting
   to Sesame’s social callback with the same `redirect_uri` and `X-Tenant-ID`.

### Common failures

| Symptom | Likely cause |
|---|---|
| `503 oauth_not_configured` | Missing DB row, `enabled=false`, or unknown tenant |
| `500` / `oauth_secret_unavailable` | Secret env var missing or empty in the pod |
| `400 redirect_uri_not_allowed` | Callback not in Sesame allowlist (exact match) |
| Google `redirect_uri_mismatch` | Callback not in Google Console (exact match) |
| `400 invalid_state` | Redis down, state expired, or tenant/provider mismatch |
| `400 email_not_verified` | Google account email not verified |
| `409 account_exists_link_required` | Email already registered; account linking required |

## Rotation

1. Create a new client secret in Google Cloud (or a new OAuth client).
2. Update the Kubernetes/secret store value for
   `SESAME_OAUTH__{TENANT}__GOOGLE_CLIENT_SECRET` (and client id if rotated).
3. Roll identity-login-service.
4. Record rotation in Sesame (`POST …/oauth/google/rotate` or CLI
   `tenant oauth rotate`) so `config_version` / audit trails update.
5. Revoke the old Google secret after validation.

## Worked example (dogfood tenant)

Replace with your tenant when publishing externally; this matches the current
dev seed for the `hauliage` tenant.

| Item | Value |
|---|---|
| Tenant slug | `hauliage` |
| Client id env | `SESAME_OAUTH__HAULIAGE__GOOGLE_CLIENT_ID` |
| Client secret env | `SESAME_OAUTH__HAULIAGE__GOOGLE_CLIENT_SECRET` |
| Example redirects | `https://hauliage.dev.microscaler.local/oauth/callback`, `http://localhost:7174/oauth/callback`, `http://127.0.0.1:7174/oauth/callback` |

```text
SESAME_OAUTH__HAULIAGE__GOOGLE_CLIENT_ID=.....apps.googleusercontent.com
SESAME_OAUTH__HAULIAGE__GOOGLE_CLIENT_SECRET=.....
```

## Security checklist

- [ ] One Google OAuth client per tenant (no shared secret across tenants)
- [ ] Secrets only in a secret store → pod env
- [ ] Redirect allowlists identical in Google Console and Sesame
- [ ] Product uses BFF pattern (no SPA holding the Google client secret)
- [ ] Consent screen and branding appropriate for production before leaving Testing
