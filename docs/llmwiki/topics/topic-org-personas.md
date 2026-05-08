---
title: Org Personas
status: partially-verified
updated: 2026-01-22
sources: [design-doc.md, sesame-idam-complete.md]
---

# Three Organization Personas

Sesame supports three distinct organization types coexisting on the same platform:

| Persona | Role | Example |
|---------|------|---------|
| **Platform** | SaaS operator | Sesame-IDAM itself |
| **Provider** | Delivers services through the platform | Employment agency, transporter |
| **Consumer** | Consumes services | Employing company, shipper |

The `org_type` claim in every JWT determines access rules:
- **Provider orgs** can see their own data plus data shared with their consumer orgs
- **Consumer orgs** can only see their own org's data
- **Platform admins** see everything

## Code Anchors

- `microservices/idam/org-mgmt/impl/src/` — Org type handling
- `openapi/org-mgmt/openapi.yaml` — Org type in API

## Gaps / Drift

> **Open:** Verify org_type handling and access rules in implementation.
