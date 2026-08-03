# Quickstart: background worker

## Goal

A worker process calls Sesame or product APIs with a user/service bearer token
without a browser session.

## Patterns

1. **Delegated user job** — enqueue `access_token` (short-lived) or a refresh
   handle held in a secret store; refresh via `https://api.<zone>/oauth/token`
   with `grant_type=refresh_token` before expiry.
2. **M2M** — out of profile v1 public consumer contract for interactive OIDC;
   use registered API-key / client-credentials surfaces documented separately.

## Rules

- Validate tokens the same way as [api-resource-server.md](./api-resource-server.md).
- Never log tokens, refresh tokens, or authorization codes.
- Honor `Retry-After` / rate-limit headers from [transport-policy-v1.md](../transport-policy-v1.md).
- Use tenant-consumer OpenAPI paths only (`/idam/v1/...` on the public API host).
