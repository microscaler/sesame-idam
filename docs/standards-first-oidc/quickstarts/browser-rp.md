# Quickstart: browser relying party

## Goal

A SPA starts login on the hosted auth origin and finishes through a BFF that
holds the client secret (if confidential) or completes PKCE (if public).

## Public hosts only

| Step | Host |
|---|---|
| Discover | `GET https://id.<zone>/.well-known/openid-configuration` |
| Authorize | Redirect to `https://auth.<zone>/oauth/authorize` |
| Token | BFF `POST https://api.<zone>/oauth/token` |
| Tenant API | `https://api.<zone>/idam/v1/...` per tenant-consumer OpenAPI |

## Required authorize parameters

`response_type=code`, `client_id`, exact `redirect_uri`, `scope=openid profile email`,
`state`, `nonce`, `code_challenge` (`S256`), `code_challenge_method=S256`.

## After tokens

1. Validate ID token (RP): `iss`, `aud`/`azp`, `nonce`, `exp`, EdDSA.
2. Map access token → verified principal only after RS validation (or BFF
   validation) using [verified-principal-mapping-v1.md](../verified-principal-mapping-v1.md).
3. Pre-org users have `organization_id: null` — still a valid session.

## Do not

- Call ClusterIP service names
- Send `X-Tenant-ID` to select tenant
- Trust flat top-level `roles` arrays
