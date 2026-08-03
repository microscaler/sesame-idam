# Verified principal mapping v1

Status: normative  
Profile version: `1.0.0`  
Schema: [verified-principal-v1.schema.json](./verified-principal-v1.schema.json)

Adapters produce a verified principal **only after** cryptographic validation of
an access token (or equivalent validated claim set). Mapping must not run on
decoded-but-unverified JWT JSON.

## Claim path → principal field

| JWT claim path | Principal field | Rule |
|---|---|---|
| *(constant)* | `profile_version` | Always `"1.0.0"` for this schema |
| `tenant_id` | `tenant_id` | Must equal `sx.tenant` |
| `sub` | `subject` | Must equal `user_id` |
| `client_id` | `client_id` | Registered OAuth client |
| `client_id` | `application_id` | v1: application identity is the registered client id |
| `sx.portal` | `portal` | Optional on principal when present |
| `sid` | `session_id` | Session binding |
| `ver` | `token_version` | Integer ≥ 1 |
| `org_id` | `organization_id` | JSON `null` when claim absent (pre-org) |
| `user_type` | `user_type` | Copied as-is |
| `sx.roles` | `roles` | Array; unique strings |
| `sx.permissions` | `permissions` | Array; unique strings |
| `sx.entitlements_ref` | `entitlements_ref` | `null` when absent |
| `sx.entitlements_hash` | `entitlements_hash` | `null` when absent |
| `act` | `actor` | Object or `null` |

Namespace alias: `sx` means the object at claim key
`https://sesame-idam.dev/claims`.

## Pre-organization state

When the access token omits `org_id`, the principal MUST set
`organization_id` to JSON `null`. Consumers must not invent a sentinel UUID.

## Rejection before mapping

Do not map when validation fails for issuer, audience, algorithm, typ, expiry,
tenant consistency, or subject/`user_id` mismatch. Those cases are authentication
failures, not empty principals.
