# api-keys

> Port: `:???` | OpenAPI 3.1.0 | 10 paths | 15 schemas

M2M API key lifecycle: creation, validation (personal + org-scoped), usage tracking, archiving.

## Quick Start

```bash
# Check the service
curl http://localhost:???/health

# List available API tags
# Each tag below maps to a handler group in impl/src/handlers/
```

## API Surface by Tag

### APIKeys

API key lifecycle and validation (M2M keys, service accounts)

- `DELETE /{key_id}`
- `GET /archived`
- `GET /archived/{key_id}`
- `GET /current`
- `GET /usage`
- `POST /`
- `POST /import`
- `POST /validate`
- `POST /validate/org`
- `POST /validate/personal`

## Schemas (15)

| Schema | Purpose |
|--------|---------|
| `ApiKey` | Schema type |
| `ApiKeyCreateResponse` | Schema type |
| `ApiKeyListResponse` | Schema type |
| `ApiKeyUsageResponse` | Schema type |
| `ApiKeyValidationResponse` | Schema type |
| `ArchivedApiKey` | Schema type |
| `ArchivedApiKeyListResponse` | Schema type |
| `CreateApiKeyRequest` | Schema type |
| `Error` | Schema type |
| `ImportApiKeysRequest` | Schema type |
| `ImportApiKeysResponse` | Schema type |
| `OrgApiKeyValidationResponse` | Schema type |
| `PersonalApiKeyValidationResponse` | Schema type |
| `UpdateApiKeyRequest` | Schema type |
| `ValidateApiKeyRequest` | Schema type |

## Codegen

This spec is the source of truth. Generated code lives in `gen/` and is rebuilt via:

```bash
just gen-api-keys
```

**Never edit files under `gen/` directly** — they are overwritten on next regeneration. Fix the OpenAPI spec instead.
