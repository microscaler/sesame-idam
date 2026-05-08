---
title: Two User Types
status: partially-verified
updated: 2026-01-22
sources: [design-doc.md, sesame-idam-complete.md]
---

# Two User Types: Customer vs Platform

## Overview

Sesame uses a single `User` table with a `user_type` column distinguishing two personas:

| Type | Who | JWT Claim | Use Case |
|------|-----|-----------|----------|
| `customer` | End users of the application | `user_type: "customer"` | B2B SaaS — users in orgs |
| `platform` | App internal users (admins, support) | `user_type: "platform"` | App admins, support, editors |

**One user table. Two JWT claim shapes. One system.**

## JWT Claim Differences

Customer JWT includes org context (org_id, org_name, user_role in that org). Platform JWT includes platform-level permissions and no org context.

## Code Anchors

- `microservices/idam/identity-login-service/impl/src/` — User type handling
- `openapi/identity-login-service/openapi.yaml` — User type in request/response

## Gaps / Drift

> **Open:** Verify actual user_type handling in implementation vs design doc.
