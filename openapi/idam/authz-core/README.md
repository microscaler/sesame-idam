# authz-core

> Port: `:8102` | OpenAPI 3.1.0 | 4 paths | 8 schemas

Centralized authorization engine. Evaluates principal permissions at request time via /principal/effective.

## Quick Start

```bash
# Check the service
curl http://localhost:8102/health

# List available API tags
# Each tag below maps to a handler group in impl/src/handlers/
```

## API Surface by Tag

### Principal

Principal role assignment, attributes, effective rights, and authorization checks

- `DELETE /principals/roles`
- `POST /authorize`
- `POST /principal/effective`
- `POST /principals/attributes`
- `POST /principals/roles`

## Schemas (8)

| Schema | Purpose |
|--------|---------|
| `AssignPrincipalRoleRequest` | Schema type |
| `AuthorizeRequest` | Schema type |
| `AuthorizeResponse` | Schema type |
| `EffectiveRequest` | Schema type |
| `EffectiveResponse` | Schema type |
| `ErrorResponse` | Schema type |
| `RevokePrincipalRoleRequest` | Schema type |
| `SetPrincipalAttributeRequest` | Schema type |

## Codegen

This spec is the source of truth. Generated code lives in `gen/` and is rebuilt via:

```bash
just gen-authz-core
```

**Never edit files under `gen/` directly** — they are overwritten on next regeneration. Fix the OpenAPI spec instead.
