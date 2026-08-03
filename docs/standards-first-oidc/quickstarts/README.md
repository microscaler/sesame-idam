# Consumer quickstarts

Executable outlines for integrating Sesame using **only** public hosts and the
portable contract. Do not configure internal service DNS or microservice OpenAPI.

## Artifacts

| Artifact | Path |
|---|---|
| Provider profile | [../provider-profile-v1.md](../provider-profile-v1.md) |
| Public OpenAPI | [`openapi/idam/tenant-consumer/openapi.yaml`](../../../openapi/idam/tenant-consumer/openapi.yaml) |
| Fixtures | [`conformance/oidc-v1/`](../../../conformance/oidc-v1/) |
| Transport | [../transport-policy-v1.md](../transport-policy-v1.md) |

## Journeys

1. [browser-rp.md](./browser-rp.md) — SPA → hosted auth → BFF callback
2. [bff.md](./bff.md) — confidential client on the BFF
3. [api-resource-server.md](./api-resource-server.md) — validate access tokens
4. [worker.md](./worker.md) — background worker with bearer token

Public hosts (replace zone as needed):

- `https://id.<zone>` — issuer, discovery, JWKS
- `https://auth.<zone>` — `/oauth/authorize`
- `https://api.<zone>` — `/oauth/token`, `/oauth/userinfo`, `/idam/v1/*`
