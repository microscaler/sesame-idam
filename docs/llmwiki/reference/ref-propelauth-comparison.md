---
title: PropelAuth Comparison
status: partially-verified
updated: 2026-01-22
sources: [sesame-idam-complete.md, docs/propelauth-gap-analysis.md]
---

# PropelAuth vs Supabase vs Sesame

## Feature Comparison

| Feature | **PropelAuth** | **Supabase Auth** | **Sesame-IDAM (Target)** |
|---------|---------------|-------------------|--------------------------|
| Core Promise | "Auth for your SaaS product" | "Auth for your database" | "Auth for your SaaS with database-level security" |
| User/Org Model | Users + Organizations | Flat Users | **Users + Organizations (B2B native)** |
| B2B Logic | **Built-in** (Invites, Seats, Roles) | Manual / Custom Logic | **Built-in (Same as PropelAuth)** |
| Database Security | JWT Claims (App logic only) | **Native RLS** | **Native RLS Helpers (We provide the SQL)** |
| Integration | Backend API + Frontend SDK | SDK + PostgREST/RLS | **Backend API + SDK + RLS Helper SQL** |
| Custom Metadata | User/Org Metadata | User Metadata | **User + Org Metadata** |
| Source | Proprietary / Paid | Open Source | **Open Source (Rust/TS)** |
| Hosted UI | Yes (Customizable) | Yes | **SDK Components (React/Vue/Plain HTML)** |

## Why Sesame Wins

Combines the best of both worlds:
- **The B2B complexity of PropelAuth** (orgs, invites, roles, seat management)
- **The database-native security of Supabase** (RLS helpers that lock down your data automatically)
- **Open source and self-hosted** (no vendor lock-in, no per-user pricing)

## Code Anchors

- `docs/propelauth-gap-analysis.md` — Gap analysis between Sesame and PropelAuth
- `docs/propelauth-api-footprint.md` — PropelAuth API footprint comparison
- `docs/propeleauth-footprint-and-developer-contract.md` — Developer contract comparison

## Gaps / Drift

> **Note:** This comparison reflects the design target. Verify implementation completeness against PropelAuth feature set.
