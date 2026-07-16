# Sesame-IDAM Pact Mock Server

Local contract-test infrastructure for identity federation — **no public internet required**.

Replaces DummyIDP / hosted SSOReady for desktop dev (NFS + ms02 Kind). DummyIDP needs a publicly reachable ACS URL; this broker binds to `127.0.0.1` or cluster ingress on the LAN.

## Quick start (ms02)

```bash
cd ~/Workspace/microscaler/seasame-idam/microservices
cargo run -p pact-mock-server --bin sesame-idam-broker
```

Default: `http://127.0.0.1:9190`

| Env | Default | Purpose |
|-----|---------|---------|
| `SESAME_BROKER_PORT` | `9190` | Listen port |
| `SESAME_BROKER_BASE_URL` | `http://127.0.0.1:9190` | URLs returned to Sesame |
| `SESAME_BROKER_APP_REDIRECT_URL` | `http://hauliage.dev.microscaler.local/saml/callback` | Post-SAML app callback |
| `SESAME_BROKER_API_KEY` | `ssoready_sk_test` | Bearer token for `/v1/saml/*` |

## SAML / SSO (SSOReady-compatible)

| Endpoint | Purpose |
|----------|---------|
| `POST /v1/saml/redirect` | Start login → `{ "redirectUrl": "…/idp/login?session=…" }` |
| `POST /v1/saml/redeem` | Exchange `samlAccessCode` → `{ email, organizationId, organizationExternalId }` |
| `GET /idp/login?session=…` | Browser IdP form (local HTML) |
| `POST /idp/simulate` | **CI** — no browser: `{ organizationExternalId, email }` → access code |
| `POST /admin/organizations` | Register test orgs |

### CI simulate flow

```bash
# 1. Issue access code
curl -s -X POST http://127.0.0.1:9190/idp/simulate \
  -H 'Content-Type: application/json' \
  -d '{"organizationExternalId":"a1000001-0001-4000-8000-000000000001","email":"buyer@testshipper.local"}'

# 2. Redeem (what identity-login-service will call)
curl -s -X POST http://127.0.0.1:9190/v1/saml/redeem \
  -H 'Authorization: Bearer ssoready_sk_test' \
  -H 'Content-Type: application/json' \
  -d '{"samlAccessCode":"<code from step 1>"}'
```

**Default test org:** `a1000001-0001-4000-8000-000000000001` with user `buyer@testshipper.local`.

## OAuth mocks (Google / Microsoft)

Point Sesame OAuth client URLs at the broker in test env:

| Real URL | Mock URL |
|----------|----------|
| `https://accounts.google.com/o/oauth2/v2/auth` | `http://127.0.0.1:9190/mock/google/o/oauth2/v2/auth` |
| `https://oauth2.googleapis.com/token` | `http://127.0.0.1:9190/mock/google/token` |
| `https://openidconnect.googleapis.com/v1/userinfo` | `http://127.0.0.1:9190/mock/google/userinfo` |
| `https://login.microsoftonline.com/common/oauth2/v2.0/authorize` | `http://127.0.0.1:9190/mock/microsoft/common/oauth2/v2.0/authorize` |
| `https://login.microsoftonline.com/common/oauth2/v2.0/token` | `http://127.0.0.1:9190/mock/microsoft/token` |
| `https://graph.microsoft.com/v1.0/me` | `http://127.0.0.1:9190/mock/microsoft/me` |

**Pact fixture codes:** `google_test_code`, `microsoft_test_code` (see `pacts/Sesame-OAuth-*.json`).

## Pact contracts

| File | Provider name |
|------|---------------|
| `pacts/Sesame-SSO-Broker.json` | `Sesame-SSO-Broker` |
| `pacts/Sesame-OAuth-Google.json` | `Google-OAuth-Mock` |
| `pacts/Sesame-OAuth-Microsoft.json` | `Microsoft-OAuth-Mock` |

The `manager` sidecar publishes these when a Pact broker + ConfigMap are present (same pattern as Hauliage Searates mocks).

## Tests

```bash
cargo test -p pact-mock-server
```

See `tests/sesame_idam_broker_integration.rs`.

## Related docs

- [design-enterprise-saml-sso.md](../../docs/design-enterprise-saml-sso.md)
- [topic-saml-implementation-maturity.md](../../docs/llmwiki/topics/topic-saml-implementation-maturity.md)
