---
title: Tooling Architecture
status: unverified
updated: 2026-05-14
sources: [PRD-SEASAME-AUDIT-REMEDIATION.md Section 6.4-6.6, brrtrouter-workspace-architecture skill]
---

# Tooling Architecture

## Current Tooling Stack

```
seasame-idam/
├── tooling/                          ← Sesame-specific tooling package
│   ├── pyproject.toml                ← Package: sesame-idam-tooling [dev]
│   └── src/sesame_idam_tooling/
│       ├── cli/main.py               ← Thin shim: delegates to brrtrouter_tooling
│       └── tilt/
│           ├── setup_kind_registry.py
│           └── setup_persistent_volumes.py
├── justfile                          ← Full justfile (init, gen, lint, serve, dev-up, tilt-*)
└── Tiltfile                          ← BROKEN — auto-generated with template failures
```

## The `sesame-idam` CLI Shim

The `sesame-idam` CLI bin (`~/.local/share/brrtrouter/venv/bin/sesame-idam`) is a thin shim:

```python
from brrtrouter_tooling.workspace.cli.main import main
```

It exposes all hauliage/BRRTRouter tooling commands through the same interface:

| Command | Example |
|---------|---------|
| `sesame-idam ports` | `sesame-idam ports list` |
| `sesame-idam openapi` | `sesame-idam openapi lint` |
| `sesame-idam ci` | `sesame-idam ci run` |
| `sesame-idam bff` | `sesame-idam bff generate` |
| `sesame-idam docker` | `sesame-idam docker copy-binary` |
| `sesame-idam gen` | `sesame-idam gen suite idam` |
| `sesame-idam build` | `sesame-idam build microservice authz-core` |
| `sesame-idam bootstrap` | `sesame-idam bootstrap init` |
| `sesame-idam release` | `sesame-idam release tag` |
| `sesame-idam tilt` | `sesame-idam tilt setup-kind-registry` |
| `sesame-idam pre-commit` | `sesame-idam pre-commit run` |

## BRRTRouter Tooling Commands (Under the Hood)

All sesame-idam CLI commands delegate to `brrtrouter_tooling.workspace`:

| Command | Implementation | Purpose |
|---------|---------------|---------|
| `sesame gen suite idam` | `brrtrouter_tooling.gen.regenerate.gen_suite` | Generate gen crates from OpenAPI specs, run fix_cargo_paths |
| `sesame gen stubs` | `brrtrouter_tooling.gen.regenerate.gen_stubs` | Regenerate impl controller stubs via brrtrouter-gen |
| `sesame build microservice` | `brrtrouter_tooling.build.workspace_build.build_microservice` | cargo zigbuild for cross-compile to Linux musl |
| `sesame docker copy-binary` | `brrtrouter_tooling.docker.copy_binary` | Copy binary to build_artifacts/ staging dir |
| `sesame docker build-image-simple` | `brrtrouter_tooling.docker.build_image_simple` | Render Dockerfile template, build image |
| `sesame docker build-base` | `brrtrouter_tooling.docker.build_base` | Build base image with dev-entrypoint.sh |
| `sesame openapi lint` | `brrtrouter_tooling.openapi.validate.validate` | Validate OpenAPI spec |
| `sesame ports list` | `brrtrouter_tooling.workspace.ports.list_ports` | List assigned ports |
| `sesame tilt setup-kind-registry` | `sesame_idam_tooling.tilt.setup_kind_registry` | Set up localhost:5001 Docker registry in Kind |
| `sesame tilt setup-persistent-volumes` | `sesame_idam_tooling.tilt.setup_persistent_volumes` | Create PVs for Redis, Postgres, etc. |

## Justfile Codegen Recipes

### Codegen per service (`just gen-<service>`)

```bash
cd BRRTRouter && cargo run --bin brrtrouter-gen -- generate \
  --spec $(pwd)/openapi/idam/<service>/openapi.yaml \
  --output $(pwd)/microservices/idam/<service>/gen \
  --package-name <service>_service_api \
  --force
```

### Lint per service (`just lint-openapi-<service>`)

```bash
cd BRRTRouter && cargo run --bin brrtrouter-gen -- lint \
  --spec $(pwd)/openapi/idam/<service>/openapi.yaml \
  --fail-on-error
```

### Serve per service (`just serve-<service>`)

```bash
cd BRRTRouter && cargo run --bin brrtrouter-gen -- serve \
  --spec $(pwd)/openapi/idam/<service>/openapi.yaml \
  --addr <addr>
```

## Dev Environment (`just dev-up`)

1. Verify shared Kind cluster exists (`kind-kind` context)
2. `sesame tilt setup-kind-registry` — set up localhost:5001 Docker registry
3. `kubectl apply -f k8s/microservices/namespace.yaml`
4. `sesame tilt setup-persistent-volumes` — Redis PVs
5. `mkdir -p /tmp/sesame-idam-data/` — host dirs for PVs
6. `tilt up --host=0.0.0.0 --port=10351`

## Supabase (`just supabase-apply`)

- `kubectl apply -k microscaler-supabase/k8s/overlays/seasame-idam`

## Port Forwarding (`just port-forward`)

- postgres: `kubectl port-forward -n data svc/postgres 5432:5432`
- redis: `kubectl port-forward -n sesame-idam svc/redis 6379:6379`

## Tilt Systemd Service (`just tilt-up/tilt-down/tilt-log`)

- Managed via `systemctl --user start tilt-sesame-idam.service`
- Port 10351

## Critical: Use Project CLI Shim, NOT `brrtrouter client`

**WRONG — mangling:** `brrtrouter client build` routes through `resolve_cargo_impl_package_name()` which transforms package names and fails.

**CORRECT:** Use the project-specific CLI shim (`sesame-idam build microservice <name>`) which delegates to `build_package_with_options()` → direct `cargo -p` with ZERO mangling.

See `brrtrouter-workspace-architecture` skill for full details on build tool mangling.

## Code Anchors

- `PRD-SEASAME-AUDIT-REMEDIATION.md Section 6.5` — Full command delegation map
- `PRD-SEASAME-AUDIT-REMEDIATION.md Section 6.4` — Justfile working commands
- `brrtrouter-workspace-architecture` skill — Build tool mangling reference
