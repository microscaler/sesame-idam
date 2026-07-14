/// Controller handlers for Organization management (SSO/SCIM, webhooks, roles).
///
/// Each controller corresponds to a single API endpoint. Controllers audit every
/// request via the global `EMITTER`, then delegate to the service layer.

pub mod list_applications;
pub mod list_my_memberships;
pub mod create_application;
pub mod create_organization;
pub mod get_application;
pub mod list_permissions;
pub mod create_permission;
pub mod list_roles;
pub mod create_role;
pub mod get_role;
pub mod get_role_permissions;
pub mod assign_permission_to_role;
pub mod revoke_permission_from_role;
pub mod query_orgs;
pub mod fetch_org;
pub mod update_org;
pub mod delete_org;
pub mod accept_invitation;
pub mod add_user_to_org;
pub mod allow_org_saml;
pub mod change_user_role_in_org;
pub mod create_saml_link;
pub mod disallow_org_saml;
pub mod update_org_domains;
pub mod enable_saml;
pub mod invite_user_to_org;
pub mod invite_user_to_org_by_id;
pub mod migrate_org_isolated;
pub mod set_oidc_idp_metadata;
pub mod revoke_pending_invite;
pub mod remove_user_from_org;
pub mod fetch_role_mappings;
pub mod delete_saml;
pub mod set_saml_idp_metadata;
pub mod fetch_scim_groups;
pub mod fetch_scim_group;
pub mod subscribe_org_to_role_mapping;
pub mod fetch_users_in_org;
pub mod fetch_webhook_subscriptions;
pub mod preview_invitation;
pub mod delete_webhook_subscription;
pub mod test_webhook_delivery;
pub mod scim_create_user;
pub mod scim_delete_user;
pub mod scim_list_users;
pub mod scim_update_user;
