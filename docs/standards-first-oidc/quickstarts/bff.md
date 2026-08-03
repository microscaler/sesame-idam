# Quickstart: BFF confidential client

## Goal

A backend-for-frontend owns the OAuth client secret, exchanges codes, stores
refresh tokens server-side, and calls the tenant-consumer API with the user
access token.

## Flow

1. Browser hits BFF `/login` → BFF redirects to authorize URL (public auth host).
2. Callback hits BFF `/callback` with `code` + `state`.
3. BFF `POST https://api.<zone>/oauth/token` with `grant_type=authorization_code`,
   PKCE verifier, and client authentication (`client_secret_basic` or `_post`).
4. BFF stores refresh token; sets a first-party session cookie for the app origin.
5. BFF proxies tenant-consumer calls to `https://api.<zone>/idam/v1` with
   `Authorization: Bearer <access_token>`.

## Contract references

- Token + error semantics: [transport-policy-v1.md](../transport-policy-v1.md)
- Operations: `openapi/idam/tenant-consumer/openapi.yaml`
- Idempotency: send `Idempotency-Key` on `POST /organizations` and invites

## Register (pre-org)

`POST /idam/v1/auth/register` with `client_id` that binds tenant. Response
`organization_id` may be null.
