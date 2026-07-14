# Epic 10: Platform Tenancy (SaaS-of-SaaS)

> **Status:** P1 PRD ready — implementation not started  
> **Design source:** [design-saas-of-saas-multi-tenancy.md](../../design-saas-of-saas-multi-tenancy.md)  
> **ADRs:** [ADR-004](../../ADR-004-platform-tenant-provisioning.md), [ADR-002](../../ADR-002-tenant-consumer-idam-api-boundary.md)

## Summary

Tenant registry, platform-admin minting, self-service provisioning (store → Stripe → worker), platform OAuth, and KYC/billing gates for Sesame sold online.

## PRDs

| Phase | PRD | Status |
|-------|-----|--------|
| **P1** | [PRD-P1-platform-tenant-admin.md](../../PRD-P1-platform-tenant-admin.md) | **Draft — ready** |
| P2 | PRD-P2-self-service-provisioning | Not written |
| P3 | PRD-P3-online-store-kyc | Not written |
| P4 | PRD-P4-tenant-secrets-scale | Not written |

## P1 stories (implement now)

| Story | Title | Status |
|-------|-------|--------|
| [10.1](./stories/story-10.1.md) | Platform OpenAPI spec + codegen | Not started |
| [10.2](./stories/story-10.2.md) | Create + get tenant | Not started |
| [10.3](./stories/story-10.3.md) | Tenant status PATCH | Not started |
| [10.4](./stories/story-10.4.md) | OAuth metadata PUT | Not started |
| [10.5](./stories/story-10.5.md) | OAuth rotate + audit | Not started |
| [10.6](./stories/story-10.6.md) | CLI tenant commands | Not started |
| [10.7](./stories/story-10.7.md) | Platform service auth | Not started |
| [10.8](./stories/story-10.8.md) | BDD mint → auth | Not started |

**Build order:** 10.1 → 10.7 → 10.2 → 10.3 → 10.4 → 10.5 → 10.6 → 10.8

## P2+ stories (backlog)

| ID | Title | Phase |
|----|-------|-------|
| 10.9 | `tenant_provisioning_jobs` entity | P2 |
| 10.10 | `POST /platform/tenants/provision` | P2 |
| 10.11 | Provisioning worker skeleton | P2 |
| 10.12 | Default org on provision | P2 |
| 10.13 | Platform admin user on provision | P2 |
| 10.14 | `tenant_subscriptions` entity | P2 |
| 10.15 | BDD provision → register | P2 |
| 10.16–10.21 | Store, KYC, billing | P3 |

See [design doc §18](../../design-saas-of-saas-multi-tenancy.md#18-story-backlog-epic-ready).

## Foundation (implemented)

- `tenants` + `tenant_oauth_providers` models and migrations
- `TenantService::require_active` on all auth entry points
- Dev seed: hauliage + pricewhisperer
- [topic-platform-tenants.md](../../llmwiki/topics/topic-platform-tenants.md)

## P1 acceptance gate

All items in [PRD-P1 §11](../../PRD-P1-platform-tenant-admin.md#11-acceptance-gate-p1-complete).
