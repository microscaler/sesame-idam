# DESIGN: Octopilot build and release

Status: in progress (October 2026). Owner: Charles Sibbald.

Sesame-IDAM builds, tests and releases through the shared Octopilot pipeline, the same way PriceWhisperer does: one
workflow from a commit on `main` to release images, a `v*` tag cutting a release, and an acceptance stage that promotes
the release into an environment where it is deployed and tested. This document is the target and the checklist. The
deployment environments themselves are described in the infrastructure repository, not here.

## 1. What Sesame ships

| Artifact | Built from | Notes |
| --- | --- | --- |
| six service images: `sesame-idam-{identity-login-service, identity-session-service, identity-user-mgmt-service, api-keys, authz-core, org-mgmt}` | one Rust workspace build (`microservices/`), fanned out per binary | BRRTRouter services; each loads its OpenAPI spec, docs and static site from its crate's `gen/` and its defaults from `impl/config` |
| `sesame-idam-migrations` | `migrations/` (apply order), `sql/rls`, the services' seeds | the schema as a release artifact, run once per release before the services roll out |
| chart `sesame-idam-microservice` | `helm/sesame-idam-microservice` | one chart for every service, values per service |
| frontend images: `auth`, `platform`, `tenant`, `brochure` | `frontend/` (pnpm workspace) → nginx | ADR-010 / ADR-013 hosts |
| chart `sesame-idam-frontend`, chart `sesame-idam-api-edge` | `helm/` | with the public entry (ADR-013) |

Not shipped: `pact-mock-server` (dev-only broker and mocks), `ui/` (superseded), the dev `db-init` job (database and
role creation belong to the platform in every environment that is not a laptop).

`sesame-idam-client` is a library, not a deployable: its pipeline is lint, test and a version tag; consumers pin a
revision.

## 2. Preconditions (done)

- **A clean checkout builds.** Microscaler crates are pinned git revisions (BRRTRouter, lifeguard, may_postgres), not
  `../../` paths; the six services share lifeguard's may_postgres. Local co-development keeps working through the
  ancestor cargo config that patches those URLs to sibling checkouts (2026-10-10, `dac59a4`).
- **Current BRRTRouter.** `HandlerRequest.raw_query` / `.span` and `HandlerResponse.sse` (2026-10-10).
- **rustfmt clean.** The pipeline's lint blocks on `cargo fmt --check` (2026-10-10, `2f5ad6b`).
- **The schema applies from scratch, twice, without a superuser** (2026-10-10). `apply_order.txt` listed four generated
  files that were never committed and are superseded by `20260802090000_oidc_client_registry_policy.sql` (which creates
  those tables with the constraints it relies on); its CHECK constraints are now added only when absent, and
  `rp_directory` grants its view owner CREATE on the schema. The Lifeguard models still generate different DDL for the
  relying-party tables than that migration: the migrator re-emits them until the two agree.

## 3. Pipeline

### 3.1 Workflows

- `.github/workflows/ci.yml` calls `octopilot/actions/.github/workflows/pipeline.yml`: detect contexts, lint (rustfmt
  blocking, clippy reported, pre-commit), test, integration builds into the integration registry, release on `v*` tags,
  acceptance promotion on `v*` tags.
- `.github/workflows/release.yml` ("Trigger Release"): bumps `VERSION`, commits, tags `v<version>`; the tag run builds and
  releases.
- `main` builds skip release commits (`VERSION` only) and changes that do not reach an image (docs, deployment
  configuration, Tilt files).
- The current `ci.yaml` is retired. Its failures predate this work and are environmental (an unpinned `just` download
  returning 403, `just init` expecting a sibling BRRTRouter checkout); its OIDC conformance and OpenAPI checks come back as
  pipeline test and lint steps.

### 3.2 Runners: public repository

Sesame is public. Self-hosted runners must never run code from a public repository's pull requests. ci.yml has no
`pull_request` trigger (push to main, `v*` tags and dispatch only), so its build jobs run on the ms02 self-hosted runners
(shared sccache, the release-notes LLM on the LAN); pull requests, forks included, run pr.yml on GitHub-hosted runners
(`ubuntu-24.04`) with no credentials, and the repository requires approval for every outside contributor's workflow
run. Promotion (acceptance) stays on GitHub-hosted runners. Credentials are federated (OIDC → cloud workload identity), never stored
as secrets; pull requests from forks get no OIDC token and so cannot push or promote. The acceptance stage runs only for
`v*` tags, which only administrators can create (tag ruleset), inside the `pp` GitHub environment.

### 3.3 Images

- **Suite build.** One Cloud Native Buildpacks build of the workspace with the Octopilot Rust builder
  (`rust-builder-a954ea2` or later: earlier builders detected Python first). `BP_RUST_PACKAGES` lists the six service
  packages; the pipeline fans the suite out to one image per binary (`/cnb/process/<package>`).
- **Kept files.** The buildpack ships only `bin/` unless told otherwise. `BP_RUST_KEEP` keeps, per service,
  `gen/doc`, `gen/static_site` and `impl/config`. The spec path is `CARGO_MANIFEST_DIR/../gen/doc/openapi.yaml`
  (`impl/../gen/doc`); the kernel resolves `..` only through an existing directory, so `impl/` must exist in the image
  (PriceWhisperer v0.1.5 lesson).
- **Paths.** The services take `--spec`, `--doc-dir`, `--static-dir` and `--config`. Dev images pass `/app/...` in their
  entrypoint; release images start at `/cnb/process/<binary>`, so the chart passes them (`workload.args`, pointing at
  `/workspace/microservices/idam/<service>/gen`) and mounts the config at `/app/config` (`workload.workingDir: /app`).
- **Migrations image.** `docker/migrations/Dockerfile` (Postgres client image + the SQL) and a `sesame-migrate` script:
  applies `migrations/apply_order.txt` (RLS contract included) and the seeds listed for every environment (see 3.4),
  as the environment's migration login (never a superuser), recording each file in a ledger table. Every file is
  applied on every release, so every file must be re-runnable. Roles outside the database (`rp_directory_owner`,
  `rp_directory_read`) are created by the platform, which grants `rp_directory_owner` to the environment's owner group. Run by the chart as
  a pre-install/pre-upgrade hook Job (`workload.kind: job`), so a release rolls out only onto its own schema.
- **Frontends.** `docker/frontend/Dockerfile` with `APP` as a build argument, one artifact per app. Pinned base images
  (`node:22-alpine`, `nginx:1.27-alpine` by digest), `pnpm install --frozen-lockfile`.

### 3.4 Seeds are not all equal

`seed_order.txt` mixes data every environment needs (platform tenants) with dev and demo data (hauliage demo users,
roles and organisations). A release must not seed demo users into a real environment. `seed_order.txt` is generated
(FK-safe order), so the classification lives beside it: `seed_environments.txt` lists the seeds every environment gets;
everything else is dev-only. The migration image contains and applies only the listed seeds; demo seeds never enter it
(dev applies them with `scripts/setup-db.sh`).

### 3.5 Secrets

Nothing secret is built into an image or passed to a build. Runtime secrets reach the services from the environment:

- passwords and API credentials: SOPS-encrypted `*.secrets.env` in `deployment-configuration` for dev; the
  environment's secret store elsewhere;
- **signing keys: born in the secret backend, never in git, not even encrypted (ADR-006)**; generate-if-absent per
  environment, so a dev key can never sign a token another environment accepts;
- the SMS credential KEK (ADR-009): per environment, generated, never shared with dev.

Pre-commit gains a SOPS check (as PriceWhisperer): a `*.secrets.env` or `*.secret.yaml` without SOPS metadata cannot be
committed.

### 3.6 Chart defaults

The chart's default `config.yaml` must carry nothing environment-specific or secret: no static API keys (the dev
`ApiKeyHeader` key), no dev issuer. Environments set `security.jwks.*` and API keys from their own values and secrets.

### 3.7 Deployment identity is configuration

What identifies a deployment to its relying parties is environment configuration, never code, so one image serves every
environment and the repository stays product-neutral: the issuer (`SESAME_JWT_ISSUER`), the accepted issuers and
audiences (`JWT_ALLOWED_ISSUERS`, `JWT_EXPECTED_AUDIENCES`), the public URLs, and the **claim namespace**
(`SESAME_CLAIMS_NAMESPACE`): the key under which the authorization claims (`sx`) travel in an access token, a URI the
deployment controls. The issuer and every consumer of its tokens (services, `sesame-idam-client`, the RLS bridge) read
the same value; unset, it is `https://sesame-idam.dev/claims`. A token whose authorization claims sit under another key
carries none for that deployment and is rejected.

## 4. Order of work

1. `VERSION`, `release.yml`, `ci.yml`, `skaffold.yaml` (suite, chart, migrations); `ci.yaml` retired; pre-commit rustfmt +
   SOPS. Platform: the acceptance identity trusts `repo:microscaler/sesame-idam:environment:pp`; the `pp` environment and
   the tag ruleset on this repository; the release token secret.
2. Migrations image and seed classification (3.4).
3. First Octopilot release (the next tag after `v0.1.2`), promoted to pp and deployed there (services, migrations). Frontends and the API edge follow
   with the public entry.
4. Acceptance tests in the pipeline (the OIDC conformance suite against the deployed release), replacing human checks.
5. `sesame-idam-client`: Octopilot lint/test/tag pipeline.

## 5. Open

- Frontend build-argument support in the pipeline release in use (PriceWhisperer pins the Octopilot `op` image from
  `main` for this).
- Issuer and hosts per environment follow ADR-013 (`id.`, `auth.`, `api.`, two consoles); the non-production names are
  decided with the public entry.
