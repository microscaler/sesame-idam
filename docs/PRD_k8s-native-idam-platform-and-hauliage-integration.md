# PRD: Kubernetes-Native Sesame-IDAM Platform + Hauliage Integration

**Date:** 2026-07-07  
**Status:** Draft — ready for review  
**Authors:** Platform team (Sesame-IDAM + Hauliage)  
**Related:**

- Hauliage: [`hauliage/docs/PRD_k8s-native-bff-routing.md`](../../hauliage/docs/PRD_k8s-native-bff-routing.md) (8080/8080 ClusterIP precedent)
- Hauliage: [`hauliage/docs/llmwiki/topics/sesame-idam-brrtrouter-integration.md`](../../hauliage/docs/llmwiki/topics/sesame-idam-brrtrouter-integration.md)
- Sesame: [`docs/llmwiki/topics/topic-tenancy-model.md`](./llmwiki/topics/topic-tenancy-model.md)
- Cluster: [`shared-k8s-cluster`](../../shared-k8s-cluster/) (platform Postgres in namespace `data`)
- BRRTRouter: [`docs/llmwiki/topics/sesame-idam-workarounds-cleanup.md`](../../BRRTRouter/docs/llmwiki/topics/sesame-idam-workarounds-cleanup.md) (BR-2 JWT claims)

---

## 1. Executive Summary

Sesame-IDAM is Microscaler’s **identity and access management (IDAM) SaaS** — a separate product from Hauliage, hosted on the same **shared-k8s** cluster. Hauliage consumes Sesame for login, JWT issuance, and JWKS validation.

Today Sesame is still shaped for **Kind-era local dev**:

- Six microservices on **unique host ports 8101–8106**
- Helm **`serviceType: NodePort`** with per-service `nodePort` assumptions
- Tilt **`port_forwards=['8101:8101', …]`** for every service
- Database on shared Postgres is **implemented** (`database sesame_idam`) but **K8s wiring is incomplete** (`database-env.yaml` missing)
- JWT signing key **misaligned** with published JWKS (`kid: dev-ephemeral` vs `key-2026-07-02-18`)
- **No role-split demo users** for parallel browser testing (`/shipper` vs `/transport`)

This PRD defines the **Kubernetes-native Sesame platform** (mirroring Hauliage’s 8080/8080 treatment), **formal database ownership** on `data` namespace Postgres, **Tilt/Justfile simplification** now that shared-k8s replaces NodePort matrices, and **end-to-end auth** with Hauliage including dual demo personas.

---

## 2. Problem Statement

### 2.1 Current behaviour

| Layer | Today | Problem |
|-------|--------|---------|
| **Helm** | Per-service ports 8101–8106; `serviceType: NodePort` | Not GKE-shaped; conflicts with cluster-wide 8080 convention |
| **Tilt** | `port_forwards=['8101:8101', …]` on all 6 services | Hides broken in-cluster DNS; ms02 LAN clutter |
| **OpenAPI `servers:`** | `http://localhost:8101`, … | Host-port era; wrong for ClusterIP |
| **Justfile** | `serve-identity-login addr="0.0.0.0:8101"` | Duplicates k8s routing |
| **Postgres** | `scripts/setup-db.sh` creates `sesame_idam` DB | Works on ms02; **no** `k8s/microservices/database-env.yaml` like Hauliage |
| **Hauliage consumer** | Helm URLs use `:8101` / `:8105` | Must move to `:8080` when Sesame migrates |
| **Auth E2E** | Login returns token; protected routes **401** | JWKS `kid` mismatch; frontend missing Bearer on most calls |
| **Demo users** | `owner@hauliage.dev` only | Cannot test shipper vs transport in two browsers |

### 2.2 Why now

- **shared-k8s-cluster** is live on ms02; platform Postgres in `data` already hosts both `hauliage` and `sesame_idam` databases.
- **Hauliage Phase 3** (BFF-only `:8080`) is done — Sesame must follow the same convention for cross-namespace calls.
- **Sesame-auth Wave 4** (frontend login + Bearer) is started; blocked on JWKS alignment and org resolution.
- **Role-split testing** (`shipper@amecorp.dev` / `transport@transportservices.dev`) is a product requirement for MVP demos.

---

## 3. Goals

1. **Uniform service port** — all Sesame-IDAM app Services use **8080 → 8080** (ClusterIP).
2. **No NodePort on app tier** — debugging via `kubectl port-forward` only (match Hauliage FR-02).
3. **Formal DB platform wiring** — ConfigMap + Secret for `postgres.data.svc.cluster.local` / `sesame_idam` database (mirror Hauliage).
4. **Tilt simplification** — remove per-service host port matrix; optional port-forward **login + session only** for isolated Sesame debugging.
5. **Hauliage integration URLs** — update `_sesame-idam-kubernetes.yaml` and smoke tests to `:8080` FQDNs.
6. **JWT signing ↔ JWKS alignment** — tokens verifiable by Hauliage `JwksBearerProvider`.
7. **Role-split demo personas** — sesame seeds + hauliage org seeds + org resolution from JWT `sub`.
8. **GKE portability** — same Helm/Tilt patterns as Hauliage staging path.

## 4. Non-Goals (this PRD)

| Item | Rationale |
|------|-----------|
| **Per-tenant Postgres databases** | Logical `tenant_id` isolation remains (see tenancy model) |
| **Sesame BFF / API gateway** | Hauliage BFF is the product gateway; Sesame services stay internal |
| **Replace Hauliage identity stub routes** | BFF login already calls Sesame; stub deprecation is follow-on |
| **Full BR-2 in BRRTRouter** | Required for clean typed-handler claims; interim `resolve_organization_id(req)` acceptable in Hauliage company service |
| **RLS policies in Postgres** | Planned (H1.5); not blocking 8080 migration |
| **Cloud Run frontend** | Deferred (Hauliage Q9) |

---

## 5. Target Architecture

### 5.1 Platform data (namespace `data`)

```text
postgres-primary (Service: postgres.data.svc.cluster.local:5432)
├── database: postgres     ← platform / Supabase control plane
├── database: hauliage     ← schema hauliage, role hauliage
└── database: sesame_idam  ← schema sesame_idam, role sesame_idam
                              tenant_id column on tenant-scoped tables
```

**Bootstrap:** `scripts/setup-db.sh` (role, database, schema, Lifeguard migrations, dev seeds).  
**Apply order:** platform Postgres up → `sesame-idam-db-init` (Tilt manual or script).

### 5.2 Sesame app tier (namespace `sesame-idam`)

```text
┌─────────────────────────────────────────────────────────────────┐
│  namespace: sesame-idam                                          │
│                                                                  │
│  identity-login-service    :8080  ClusterIP                      │
│  identity-session-service  :8080  ClusterIP  (JWKS, refresh)     │
│  identity-user-mgmt-service:8080  ClusterIP                      │
│  authz-core                :8080  ClusterIP                      │
│  api-keys                  :8080  ClusterIP                      │
│  org-mgmt                  :8080  ClusterIP                      │
│                                                                  │
│  redis (app-local cache)   :6379  ClusterIP                      │
└─────────────────────────────────────────────────────────────────┘
         ▲                                    │
         │  cross-namespace HTTP :8080         │ 5432
         │                                    ▼
┌────────┴────────────────────────────────────────────────────────┐
│  namespace: hauliage                                             │
│  bff :8080  ──login──► identity-login-service.sesame-idam:8080   │
│  fleet/company/consignments ──JWKS──► identity-session-service   │
│                                       .sesame-idam:8080           │
└─────────────────────────────────────────────────────────────────┘
```

**Service identity** is the **Kubernetes Service name**, not the legacy port number.  
Example login URL after migration:

```text
http://identity-login-service.sesame-idam.svc.cluster.local:8080/idam/v1/auth/login
```

### 5.3 Dev on ms02

| Path | Purpose |
|------|---------|
| **Hauliage Vite :7174 → BFF :8080** | Primary product dev; login via BFF → Sesame in-cluster |
| **Optional PF: login + session :8080** | Isolated Sesame debugging without Hauliage |
| **No PF: authz-core, api-keys, org-mgmt, user-mgmt** | ClusterIP only |
| **`just port-forward`** | Postgres `:5432`, Redis `:6379` only — **not** 8101–8106 |

### 5.4 Demo personas (role-split browsers)

| Email | Password | Hauliage portal | Org (company DB) | Sesame `sub` (seed) |
|-------|----------|-----------------|------------------|---------------------|
| `shipper@amecorp.dev` | `SecureP@ss123!` | `/shipper/*` | AME Corp (`SHIPPER`) | `a1000001-0001-4000-8000-000000000004` |
| `transport@transportservices.dev` | `SecureP@ss123!` | `/transport/*` | Transport Services (`HAULIER`) | `a1000001-0001-4000-8000-000000000005` |
| `owner@hauliage.dev` | `SecureP@ss123!` | either (legacy) | Transport Services (default) | `a1000001-0001-4000-8000-000000000001` |

---

## 6. User Stories

### Developer

- **US-D1:** As a developer, I run `just dev-up` in sesame-idam and all services listen on ClusterIP `:8080` without NodePort conflicts.
- **US-D2:** As a developer, I sign in as shipper in Chrome and transport in Firefox and see **different org profiles** on `/organizations/me`.
- **US-D3:** As a developer, I do not maintain an 8101–8106 port matrix in Tilt, Justfile, or OpenAPI.

### Platform / SRE

- **US-P1:** As platform ops, Sesame and Hauliage deploy to shared-k8s and GKE with the same 8080 convention.
- **US-P2:** As platform ops, `database-env.yaml` + Secret mirror the Hauliage pattern for onboarding new envs.
- **US-P3:** As platform ops, JWT from login validates against session-service JWKS without manual port-forwards.

### Hauliage consumer

- **US-H1:** As Hauliage BFF, I call Sesame login at in-cluster `:8080` with 30s timeout budget (bcrypt ~10s).
- **US-H2:** As Hauliage fleet/company, I fetch JWKS from session-service `:8080` inside the cluster.
- **US-H3:** As frontend, after sign-in I attach Bearer token and `/organizations/me` + `/fleet/vehicles` return **200**.
- **US-H4:** As a new user, I register an account without an org, then create a shipper or haulier workspace on `/onboarding`, or join via invite magic link (see Hauliage [`PRD_account-first-onboarding.md`](../../hauliage/docs/PRD_account-first-onboarding.md)).

---

## 5.5 Account-first onboarding (Hauliage)

Sesame provides **identity only** at register. Hauliage org workspaces are created or joined **after** authentication — no email-domain auto-provisioning and no freemail blocklists.

| Step | Sesame | Hauliage |
|------|--------|----------|
| Register | User + JWT | No org |
| Create workspace | — | `POST /organizations/me` |
| Join workspace | — | Invite magic link → `POST /organizations/me/invites/accept` |

Demo seeds (`shipper@amecorp.dev`, `transport@transportservices.dev`) retain pre-mapped orgs for role-split browser testing.

---

## 7. Functional Requirements

### 7.1 Database platform (namespace `data`)

| ID | Requirement |
|----|-------------|
| **FR-DB-01** | Database `sesame_idam`, schema `sesame_idam`, role `sesame_idam` created by `scripts/setup-db.sh` (idempotent). |
| **FR-DB-02** | Add `k8s/microservices/database-env.yaml`: ConfigMap `sesame-idam-database-config` + Secret `sesame-idam-db-credentials` in namespace `sesame-idam`. |
| **FR-DB-03** | Helm merges `values/_database-kubernetes.yaml`; all six services read `DB_HOST=postgres.data.svc.cluster.local`, `DB_NAME=sesame_idam`, `DB_USER=sesame_idam`. |
| **FR-DB-04** | `just dev-up` / Tilt applies `database-env.yaml` before Helm deploys (mirror Hauliage `hauliage-database-env.yaml`). |
| **FR-DB-05** | Document bootstrap order: shared-k8s platform → sesame namespace → `sesame-idam-db-init` → services. |
| **FR-DB-06** | Seeds include role-split users; re-run idempotent via `ON CONFLICT`. |

### 7.2 Helm chart (`sesame-idam-microservice`)

| ID | Requirement |
|----|-------------|
| **FR-Helm-01** | Default `service.port` and `service.containerPort` are **8080** for **all** Sesame services. |
| **FR-Helm-02** | Default `serviceType` is **ClusterIP**; remove `NodePort` from per-service values. |
| **FR-Helm-03** | Remove per-service `nodePort` fields from values files. |
| **FR-Helm-04** | Deployment sets `PORT=8080` from `containerPort` (verify all binaries honour `PORT` env). |
| **FR-Helm-05** | Service manifest: named port `http`, **8080 → 8080**. |
| **FR-Helm-06** | Chart README documents breaking change from 8101–8106. |

### 7.3 Tilt / Justfile

| ID | Requirement |
|----|-------------|
| **FR-Tilt-01** | Remove `IDAM_PORTS` host-port map from Tiltfile; use `SERVICE_HTTP_PORT = '8080'`. |
| **FR-Tilt-02** | Remove `port_forwards=['810x:810x']` from all six services by default. |
| **FR-Tilt-03** | Optional: port-forward **identity-login-service** and **identity-session-service** only at `8080:8080` (debug flag or comment). |
| **FR-Tilt-04** | `bundled_data_stack = False` remains; document dependency on shared-k8s `data/postgres-primary`. |
| **FR-Tilt-05** | Apply `k8s/microservices/database-env.yaml` in Tiltfile (like Hauliage). |
| **FR-Tilt-06** | Wire Redis manifest (`k8s/data/redis.yaml`) into Tilt if not already applied. |
| **FR-Just-01** | Update `just port-forward`: postgres + redis only; remove 8101/8105 host forwards from docs. |
| **FR-Just-02** | Update `serve-identity-*` recipes to default `:8080` or mark deprecated in favour of k8s. |

### 7.4 OpenAPI / codegen

| ID | Requirement |
|----|-------------|
| **FR-OAPI-01** | Per-service OpenAPI `servers:` use in-cluster form: `http://{service-name}:8080/idam/v1/...` (not localhost:810x). |
| **FR-OAPI-02** | Regenerate all `gen/` crates after server URL changes (`sesame-idam gen suite idam --service …` on **ms02**). |
| **FR-OAPI-03** | Deprecate port numbers in `openapi/idam/bff-suite-config.yaml` if present; use service name + 8080 runtime constant. |

### 7.5 JWT / JWKS alignment

| ID | Requirement |
|----|-------------|
| **FR-Auth-01** | Login-issued JWT `kid` **must** appear in session-service JWKS document. |
| **FR-Auth-02** | Dev ephemeral signer publishes `dev-ephemeral` to JWKS **or** login uses the same key id as JWKS (`key-2026-07-02-18`). |
| **FR-Auth-03** | `iss` = `https://idam.example.com`, `aud` includes `sesame-idam` (array supported by BRRTRouter validation). |
| **FR-Auth-04** | Smoke: login → Bearer → `GET /idam/v1/...` protected route returns 200 on ms02 without host port-forwards. |

### 7.6 Role-split demo data

| ID | Requirement |
|----|-------------|
| **FR-Demo-01** | Sesame seed: `shipper@amecorp.dev`, `transport@transportservices.dev` (password `SecureP@ss123!`). |
| **FR-Demo-02** | Sesame authz seed: `OWNER` role assignments for both users (tenant `hauliage`). |
| **FR-Demo-03** | Hauliage company seed: org `AME Corp` (`SHIPPER`, UUID `b2000002-…`); rename/update transport org to **Transport Services**. |
| **FR-Demo-04** | Hauliage `org_resolution`: map JWT `sub` → organization UUID (interim until BR-2). |
| **FR-Demo-05** | Document credentials in both repos’ llmwiki (replace single `owner@hauliage.dev` as sole demo user). |

### 7.7 Hauliage consumer updates

| ID | Requirement |
|----|-------------|
| **FR-Haul-01** | Update `helm/.../values/_sesame-idam-kubernetes.yaml`: all Sesame URLs use **:8080**. |
| **FR-Haul-02** | Update `microservices/bff/impl`, fleet, company, consignments local `config.yaml` dev JWKS/login URLs to `:8080` (127.0.0.1 PF targets). |
| **FR-Haul-03** | Update smoke tests (`sesame_jwks_smoke.rs`) default URLs to `:8080`. |
| **FR-Haul-04** | Frontend Wave 4 complete: sign-in forms → BFF login; `authFetch` on protected routes. |
| **FR-Haul-05** | E2E: optional real-login path with `shipper@amecorp.dev` / `transport@transportservices.dev`. |

---

## 8. Non-Functional Requirements

| ID | Category | Requirement |
|----|----------|-------------|
| **NFR-01** | Security | Sesame app Services not NodePort by default; LAN cannot reach IDAM except via Hauliage BFF or explicit PF. |
| **NFR-02** | Security | DB credentials in Secret only; dev password documented, not committed to prod overlays. |
| **NFR-03** | Reliability | Login timeout ≤30s (BFF budget); bcrypt cost documented for ms02 latency. |
| **NFR-04** | Compatibility | Coordinate Sesame + Hauliage URL port change in one release window on ms02. |
| **NFR-05** | Testability | `cargo test` smoke for login→JWKS→protected route in both repos. |
| **NFR-06** | Operability | Runbook: "401 after login" → check JWT `kid` vs JWKS keys. |
| **NFR-07** | Maintainability | Gen commands run on **ms02** only (Mac regen stalls). |

---

## 9. Acceptance Criteria

### 9.1 Database

- [ ] **AC-DB-01:** `\l` on ms02 shows `sesame_idam` and `hauliage` databases.
- [ ] **AC-DB-02:** `\dt sesame_idam.*` ≥ 33 tables after migrations + seeds.
- [ ] **AC-DB-03:** `kubectl get cm,secret -n sesame-idam` includes database config + credentials.
- [ ] **AC-DB-04:** Sesame pod connects using ConfigMap host (not hardcoded superuser).

### 9.2 Helm / cluster (8080)

- [ ] **AC-K8s-01:** All Sesame Services in `sesame-idam` expose port **8080**, targetPort **8080**, type **ClusterIP**.
- [ ] **AC-K8s-02:** No `nodePort: 310xx` in default values files.
- [ ] **AC-K8s-03:** From hauliage BFF pod: `wget -qO- http://identity-login-service.sesame-idam.svc.cluster.local:8080/health` succeeds.
- [ ] **AC-K8s-04:** From company pod: JWKS fetch at session-service `:8080` succeeds.

### 9.3 Tilt / dev workflow

- [ ] **AC-Tilt-01:** Tiltfile has no `8101:8101` style port_forwards on all services.
- [ ] **AC-Tilt-02:** `just dev-up` starts stack without NodePort collisions on ms02.
- [ ] **AC-Tilt-03:** Optional login/session PF documented in Tiltfile comments.

### 9.4 Auth E2E (Hauliage + Sesame)

- [ ] **AC-Auth-01:** `POST /api/v1/identity/auth/login` (BFF) returns `access_token`.
- [ ] **AC-Auth-02:** JWT header `kid` matches a key in JWKS.
- [ ] **AC-Auth-03:** `GET /api/v1/organizations/me` with Bearer returns **200** and correct org name per user.
- [ ] **AC-Auth-04:** `GET /api/v1/fleet/vehicles` with transport user returns **200**; shipper user behaviour documented (403 or empty per policy).
- [ ] **AC-Auth-05:** Two browsers: shipper → AME Corp dashboard; transport → Transport Services dashboard.

### 9.5 Demo seeds

- [ ] **AC-Demo-01:** Sesame `users` table contains `shipper@amecorp.dev` and `transport@transportservices.dev`.
- [ ] **AC-Demo-02:** Hauliage `organization_profiles` contains AME Corp (`SHIPPER`) and Transport Services (`HAULIER`).

---

## 10. Implementation Phases

| Phase | Scope | Repos | Depends on |
|-------|--------|-------|------------|
| **0** | PRD review + sign-off | sesame-idam, hauliage | — |
| **1** | `database-env.yaml` + Tilt apply; document bootstrap order | sesame-idam | shared-k8s platform |
| **2** | Helm 8080/8080 + ClusterIP; remove NodePorts | sesame-idam | Phase 0 |
| **3** | Tiltfile + Justfile port-forward cleanup | sesame-idam | Phase 2 |
| **4** | OpenAPI `servers:` + regen on ms02 | sesame-idam | Phase 2 |
| **5** | JWT/JWKS `kid` alignment | sesame-idam | Phase 2 |
| **6** | Role-split seeds (sesame + hauliage company) + org_resolution | sesame-idam, hauliage | Phase 5 |
| **7** | Hauliage consumer URL updates (`:8080`); redeploy BFF/fleet/company | hauliage | Phase 2–5 |
| **8** | Frontend auth Wave 4 completion + dual-browser smoke | hauliage | Phase 6–7 |
| **9** | Docs/llmwiki + GKE staging checklist | both + shared-k8s-cluster | Phase 8 |

**Parallel:** Hauliage BFF routing Phase 3 (done) does not block Sesame Phase 2.

---

## 11. Port Migration Reference

Legacy Kind/host ports → Kubernetes Service name (all TCP **8080** after migration):

| Service | Legacy port | K8s Service name | In-cluster base (after) |
|---------|-------------|------------------|-------------------------|
| identity-login-service | 8101 | `identity-login-service` | `http://identity-login-service.sesame-idam.svc.cluster.local:8080/idam/v1` |
| authz-core | 8102 | `authz-core` | `http://authz-core.sesame-idam.svc.cluster.local:8080/idam/v1` |
| api-keys | 8103 | `api-keys` | `http://api-keys.sesame-idam.svc.cluster.local:8080/idam/v1` |
| org-mgmt | 8104 | `org-mgmt` | `http://org-mgmt.sesame-idam.svc.cluster.local:8080/idam/v1` |
| identity-session-service | 8105 | `identity-session-service` | `http://identity-session-service.sesame-idam.svc.cluster.local:8080/idam/v1` |
| identity-user-mgmt-service | 8106 | `identity-user-mgmt-service` | `http://identity-user-mgmt-service.sesame-idam.svc.cluster.local:8080/idam/v1` |

**Hauliage helm URLs to update:**

| Setting | Before | After |
|---------|--------|-------|
| `sesameIdam.loginUrl` | `…:8101/idam/v1/auth/login` | `…:8080/idam/v1/auth/login` |
| `sesameIdam.jwksUrl` | `…:8105/idam/v1/.well-known/jwks.json` | `…:8080/idam/v1/.well-known/jwks.json` |
| `sesameIdam.sessionBaseUrl` | `…:8105/idam/v1` | `…:8080/idam/v1` |

---

## 12. File Change Matrix

| File | Repo | Change |
|------|------|--------|
| `helm/sesame-idam-microservice/values/*.yaml` | sesame-idam | 8080, ClusterIP |
| `Tiltfile` | sesame-idam | Remove IDAM_PORTS PF matrix |
| `justfile` | sesame-idam | port-forward, serve-* defaults |
| `k8s/microservices/database-env.yaml` | sesame-idam | **Create** |
| `openapi/idam/*/openapi.yaml` | sesame-idam | servers → `:8080` |
| `identity-user-mgmt-service/impl/seeds/*` | sesame-idam | role-split users |
| `microservices/idam/common/src/jwt/signer.rs` | sesame-idam | JWKS kid alignment |
| `helm/.../_sesame-idam-kubernetes.yaml` | hauliage | `:8080` URLs |
| `microservices/company/impl/src/org_resolution.rs` | hauliage | JWT sub → org |
| `company/impl/seeds/*` | hauliage | AME Corp + Transport Services |
| `frontend/src/**` | hauliage | authFetch (Wave 4) |
| `docs/llmwiki/topics/sesame-idam-brrtrouter-integration.md` | hauliage | demo users, troubleshooting |

---

## 13. Open Questions

| ID | Question | Default if unresolved |
|----|----------|------------------------|
| **Q1** | Port-forward login+session in Tilt by default? | **Yes** — optional PF for isolated Sesame debug |
| **Q2** | Should shipper user get fleet API access? | **No** — 403 or empty; transport only sees fleet |
| **Q3** | Deprecate `owner@hauliage.dev`? | **Keep** — maps to transport org for backward compat |
| **Q4** | Redis: sesame-idam local vs platform `data/redis`? | **Keep app-local** redis in sesame-idam for now |

---

## 14. Sign-off Checklist

- [ ] Platform: shared-k8s Postgres capacity for both SaaS DBs
- [ ] Sesame: Helm/Tilt 8080 migration plan accepted
- [ ] Hauliage: consumer URL update in same release window
- [ ] Security: JWKS alignment verified on ms02 before demo users land
- [ ] Product: shipper/transport emails and org names approved

---

*End of PRD.*
