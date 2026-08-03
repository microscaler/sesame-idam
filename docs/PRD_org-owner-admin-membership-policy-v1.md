# PRD: Organization Owner vs Admin Membership Policy

| Field | Value |
|-------|--------|
| **Version** | `v2026-08-03` |
| **Status** | Active — authoritative platform policy; product-path guards landed 2026-08-03 |
| **Owner** | Sesame-IDAM (org-mgmt) |
| **Paired consumer PRD** | [`../../hauliage/docs/PRD_loadlinker-owner-admin-roles-v1.md`](../../hauliage/docs/PRD_loadlinker-owner-admin-roles-v1.md) |
| **Related** | [`PRD_workspace-onboarding-and-org-invites-v2.md`](./PRD_workspace-onboarding-and-org-invites-v2.md) (invite/list/accept); ADR-002 |

---

## 1. Executive summary

Sesame stores org memberships with roles including `owner` and `admin`. Product consumers (Loadlinker shipper + transporter first) need a stable rule:

- **Owner** is the org principal (minted at org create).
- **Admin** is elevated org ops (may invite and remove **non-owners**).
- **Owners cannot be removed or demoted** via org-admin product APIs.
- **Owner removal / succession** is a future **customer-service / platform-admin** capability.

This PRD is the platform authority. Consumers must not invent a weaker client-only rule.

---

## 2. Problem

| Gap | Risk |
|-----|------|
| `remove_member` allowed any org owner/admin to delete any membership, including other owners | Peer-owner lockout / hostile takeover within an org |
| `change_member_role` could demote Owner to Admin/member | Same outcome without DELETE |
| Demo seeds placed two `owner` rows on Transport Services | Consumers displayed two undeletable “principals” |
| No stable error code for Owner protection | BFFs could not map UX clearly |

---

## 3. Goals

1. Authoritative product-path forbid for Owner remove and Owner demotion.
2. Stable error code **`cannot_remove_owner`** (HTTP **403**).
3. Preserve `require_org_admin` (Owner **or** Admin may manage the org).
4. Demo seeds: one Owner per demo org; document Admin demo users.
5. Leave CS privileged Owner transfer/removal as an explicit follow-up.

## 4. Non-goals

| Item | Rationale |
|------|-----------|
| Multi-owner product feature | Not supported v1 |
| Full RBAC permission catalog | Separate authz work |
| Changing invite email / accept flows | Owned by onboarding PRD v2 |
| Implementing CS API in this version | Documented only (§8) |

---

## 5. Membership roles (platform)

Stored on `sesame_idam.org_memberships.role` (lowercase slugs):

| Role | Meaning | Product create | Product remove | Product demote |
|------|---------|----------------|----------------|----------------|
| `owner` | Org principal | Org create only | **Forbidden** | **Forbidden** |
| `admin` | Elevated org ops | Invite / role change | Allowed | Allowed (to non-owner roles) |
| Other product slugs | `dispatcher`, `driver`, `member`, … | Invite / role change | Allowed | Allowed |

`require_org_admin`: caller membership role is `owner` **or** `admin`.

---

## 6. API contract (product path)

### 6.1 `DELETE /organizations/{org_id}/users/{user_id}`

After auth + org-admin check + self-remove check:

1. Load target membership.
2. If `role` equals `owner` (case-insensitive) → **403** `cannot_remove_owner`.
3. Else delete membership.

### 6.2 `PATCH /organizations/{org_id}/users/{user_id}/role`

If target membership is `owner` and new role is not `owner` → **403** `cannot_remove_owner`.

### 6.3 Error shape

```json
{
  "error": "cannot_remove_owner",
  "message": "Organization owners cannot be removed on the product path"
}
```

(Demotion message may say “cannot be demoted”; same `error` code.)

---

## 7. Demo seeds (tenant `hauliage`)

| Email | Role | Org |
|-------|------|-----|
| `shipper@amecorp.dev` | `owner` (sole) | AME Corp |
| `transport@transportservices.dev` | `owner` (sole) | Transport Services |
| `owner@hauliage.dev` | `admin` | Transport Services |
| `dispatcher@…`, `driver@…` | product roles | Transport Services |

Seed file: `microservices/idam/org-mgmt/impl/seeds/20260706000002_acme_demo_orgs.sql`.

---

## 8. Owner transfer & CS operations

**Authoritative design:** [`design-org-owner-transfer-and-ops-consoles.md`](./design-org-owner-transfer-and-ops-consoles.md).

Summary:

1. **Transfer** is the only supported succession primitive (not product DELETE of Owner).
2. **Product path:** current Owner may `POST /organizations/{org_id}/owner/transfer` after `…/transfer/challenge` email OTP (walk-away console protection). Non-Owner role changes use a simple UI confirmation only.
3. **Tenant CS path:** tenant-bound `TenantCsAuth` may `POST /cs/organizations/{org_id}/owner/transfer` (reason required; no email OTP).
4. Platform admin key remains for tenant registry only — not org Owner mutation.
5. Last-Owner invariant + audit + session invalidation are mandatory.

Product clients still treat Owner as immutable on DELETE/role-change APIs (`cannot_remove_owner`).

---

## 9. Acceptance criteria

- [ ] `remove_member` against Owner → `OrgLifecycleError::CannotRemoveOwner` → 403 `cannot_remove_owner`.
- [ ] Admin removing a non-owner succeeds.
- [ ] Peer Owner removing another Owner fails with `cannot_remove_owner`.
- [ ] Admin demoting Owner fails with `cannot_remove_owner`.
- [ ] BDD coverage for the above (±).
- [ ] Demo Transport Services has a single `owner` membership.

---

## 10. Implementation status (2026-08-03)

| Area | Status |
|------|--------|
| `OrgLifecycleError::CannotRemoveOwner` | Done |
| Controllers map to 403 `cannot_remove_owner` | Done |
| VersionStore bump only after successful remove/role change | Done |
| BDD positive/negative | Done |
| Demo seed demotion | Done |
| Owner transfer design + API (product + tenant CS) | Done — see design doc |

---

## 11. Consumer mapping

Loadlinker maps Sesame slugs to uppercase API tokens (`OWNER`, `ADMIN`) and product labels. See paired consumer PRD for shipper/transporter invite catalogs and UI rules.

**Authority:** Sesame org-mgmt is the sole product-path enforcer of Owner immutability. Consumer BFFs must call Sesame remove/role APIs and **pass through** `cannot_remove_owner` (and other Sesame `error` codes). BFFs must not re-implement membership policy by listing members and deciding locally. UI may hide Owner remove as UX only.
