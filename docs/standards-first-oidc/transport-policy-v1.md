# Transport policy v1

Status: normative  
Profile version: `1.0.0`  
Public API: [tenant-consumer OpenAPI](../../openapi/idam/tenant-consumer/openapi.yaml)

## Token response

Successful credential issuance (`TokenResponse`) includes:

| Field | Required | Notes |
|---|---|---|
| `access_token` | yes | Compact JWT |
| `refresh_token` | no | Present when refresh grant issued |
| `expires_in` | yes | Access-token lifetime seconds |
| `token_type` | yes | Always `Bearer` |
| `user_id` | yes | Same as access-token `sub` |
| `organization_id` | no | Omitted or null in pre-org state |

## Error object

Public JSON APIs use:

```json
{
  "error": "invalid_request",
  "error_description": "human-readable detail",
  "request_id": "optional correlation id"
}
```

OAuth token endpoint errors follow RFC 6749 (`error`, optional
`error_description`, optional `error_uri`). Never echo secrets, codes,
verifiers, or raw tokens in `error_description`.

| HTTP | Typical `error` |
|---|---|
| 400 | `invalid_request`, `invalid_grant` |
| 401 | `invalid_client`, `invalid_token` |
| 403 | `insufficient_scope`, `access_denied` |
| 404 | `not_found` |
| 409 | `conflict` |
| 429 | `rate_limited` |
| 503 | `temporarily_unavailable` |

## Pagination

List endpoints that can grow use cursor pagination:

| Parameter / field | Location | Notes |
|---|---|---|
| `limit` | query | Default 50, max 200 |
| `cursor` | query | Opaque; omit for first page |
| `next_cursor` | body | Null/absent when no further pages |
| `items` | body | Page payload array |

Clients must not invent offsets or scrape beyond `next_cursor`.

## Idempotency

Mutating tenant-consumer operations that create resources accept:

| Header | Required | Notes |
|---|---|---|
| `Idempotency-Key` | recommended | UUID or opaque string ≤ 128 chars |

Replays with the same key and identical body return the original success
response. Conflicting bodies with the same key return `409 conflict`.

## Retries

| Condition | Client action |
|---|---|
| `408`, `429`, `500`, `502`, `503`, `504` | Retry with exponential backoff |
| `Retry-After` present | Wait at least that many seconds |
| `4xx` other than above | Do not retry without changing the request |

Maximum recommended attempts for idempotent GETs: 3. For POSTs, retry only
when an `Idempotency-Key` was sent.

## Rate-limit headers

When rate limiting applies, responses include:

| Header | Meaning |
|---|---|
| `X-RateLimit-Limit` | Requests permitted in the window |
| `X-RateLimit-Remaining` | Remaining in the window |
| `X-RateLimit-Reset` | Unix seconds when the window resets |
| `Retry-After` | Seconds to wait (on 429) |

## Tenancy headers

Callers must **not** send `X-Tenant-ID` to select tenancy. Tenant is bound by
the registered client (public signup) or the validated bearer token. The API
origin strips caller-provided tenant override headers.

## Cookies

The public API origin strips inbound `Cookie` and outbound `Set-Cookie` for
token and JSON API routes. Hosted auth on `auth.<zone>` may use first-party
cookies for the login SPA only.
