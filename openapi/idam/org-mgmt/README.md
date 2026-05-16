# org-mgmt

> Port: `:???` | OpenAPI 3.1.0 | 34 paths | 42 schemas

Organization lifecycle, SAML/SCIM SSO, membership management, application/role/permission RBAC, webhooks, API key invalidation, and SCIM user provisioning.

## Quick Start

```bash
# Check the service
curl http://localhost:???/health

# List available API tags
# Each tag below maps to a handler group in impl/src/handlers/
```

## API Surface by Tag

### AccountSecurity

User-level security actions including API key invalidation

- `POST /admin/users/{user_id}/invalidate-all-keys`

### Applications

Register applications and define roles/permissions

- `GET /applications`
- `GET /applications/{app_id}`
- `POST /applications`

### Membership

User-org relationships, invites, roles

- `DELETE /{org_id}/pending-invites`
- `GET /{org_id}/role-mappings`
- `GET /{org_id}/users`
- `POST /{org_id}/add-user`
- `POST /{org_id}/change-role`
- `POST /{org_id}/invite-user`
- `POST /{org_id}/invite-user-by-id`
- `POST /{org_id}/remove-user`
- `PUT /{org_id}/subscribe-role-mapping`

### Organizations

Organization lifecycle (create, fetch, update, delete, query)

- `DELETE /{org_id}`
- `GET /`
- `GET /{org_id}`
- `PUT /{org_id}`
- `PUT /{org_id}/domains`

### Permissions

Permissions per application

- `GET /applications/{app_id}/permissions`
- `POST /applications/{app_id}/permissions`

### Roles

Roles per application

- `DELETE /applications/{app_id}/roles/{role_id}/permissions`
- `GET /applications/{app_id}/roles`
- `GET /applications/{app_id}/roles/{role_id}`
- `GET /applications/{app_id}/roles/{role_id}/permissions`
- `POST /applications/{app_id}/roles`
- `POST /applications/{app_id}/roles/{role_id}/permissions`

### SCIM

SCIM 2.0 user provisioning for enterprise SSO

- `DELETE /{org_id}/scim/users/{user_id}`
- `GET /{org_id}/scim/users`
- `POST /{org_id}/scim/users`
- `PUT /{org_id}/scim/users/{user_id}`

### SSO

Enterprise SAML/OIDC/SCIM per-organisation configuration

- `DELETE /{org_id}/sso/saml`
- `GET /{org_id}/scim/groups`
- `GET /{org_id}/scim/groups/{group_id}`
- `POST /{org_id}/allow-saml`
- `POST /{org_id}/create-saml-link`
- `POST /{org_id}/disallow-saml`
- `POST /{org_id}/enable-saml`
- `POST /{org_id}/migrate-to-isolated`
- `POST /{org_id}/oidc-metadata`
- `PUT /{org_id}/sso/saml-metadata`

### Webhooks

Webhook subscription management for real-time event notifications

- `DELETE /{org_id}/webhooks/{subscription_id}`
- `GET /{org_id}/webhooks`
- `POST /{org_id}/webhooks/{subscription_id}/test`

## Schemas (42)

| Schema | Purpose |
|--------|---------|
| `AddUserToOrgRequest` | Schema type |
| `Application` | Schema type |
| `ApplicationListResponse` | Schema type |
| `AssignPermissionRequest` | Schema type |
| `ChangeUserRoleRequest` | Schema type |
| `CreateApplicationRequest` | Schema type |
| `CreateOrgRequest` | Schema type |
| `CreatePermissionRequest` | Schema type |
| `CreateRoleRequest` | Schema type |
| `CreateWebhookSubscriptionRequest` | Schema type |
| `Error` | Schema type |
| `InvalidateKeysResponse` | Schema type |
| `InviteUserToOrgByIdRequest` | Schema type |
| `InviteUserToOrgRequest` | Schema type |
| `OidcMetadataRequest` | Schema type |
| `Org` | Schema type |
| `OrgDomainsRequest` | Schema type |
| `OrgListResponse` | Schema type |
| `PendingInvitesResponse` | Schema type |
| `Permission` | Schema type |
| `PermissionListResponse` | Schema type |
| `RemoveUserFromOrgRequest` | Schema type |
| `RevokeInviteRequest` | Schema type |
| `Role` | Schema type |
| `RoleListResponse` | Schema type |
| `RoleMappingResponse` | Schema type |
| `SamlConnectionLinkResponse` | Schema type |
| `SamlLinkRequest` | Schema type |
| `ScimGroup` | Schema type |
| `ScimGroupsResponse` | Schema type |
| `ScimUser` | Schema type |
| `ScimUserCreateRequest` | Schema type |
| `ScimUserListResponse` | Schema type |
| `ScimUserUpdateRequest` | Schema type |
| `SubscribeRoleMappingRequest` | Schema type |
| `UpdateOrgRequest` | Schema type |
| `UpdateWebhookSubscriptionRequest` | Schema type |
| `UsersInOrgResponse` | Schema type |
| `WebhookEvent` | Schema type |
| `WebhookSubscription` | Schema type |
| `WebhookSubscriptionListResponse` | Schema type |
| `WebhookTestResponse` | Schema type |

## Codegen

This spec is the source of truth. Generated code lives in `gen/` and is rebuilt via:

```bash
just gen-org-mgmt
```

**Never edit files under `gen/` directly** — they are overwritten on next regeneration. Fix the OpenAPI spec instead.
