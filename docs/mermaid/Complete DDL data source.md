```mermaid
classDiagram
direction BT
class api_key_rate_limits {
   uuid api_key_id
   int policy_id
}
class api_keys {
   uuid user_id
   text key_hash
   text description
   timestamptz created_at
   timestamptz expires_at
   boolean revoked
   uuid api_key_id
}
class audit_logs {
   uuid user_id
   uuid org_id
   text action
   text object_type
   text object_id
   inet ip_address
   text user_agent
   timestamptz timestamp
   jsonb details
   uuid log_id
}
class email_verification_tokens {
   uuid user_id
   timestamptz created_at
   timestamptz expires_at
   timestamptz used_at
   uuid token
}
class identity_providers {
   uuid org_id
   text type
   text name
   jsonb config
   boolean enabled
   timestamptz created_at
   uuid provider_id
}
class impersonation_logs {
   uuid admin_user_id
   uuid impersonated_user_id
   timestamptz started_at
   timestamptz ended_at
   inet ip_address
   text user_agent
   uuid log_id
}
class login_attempts {
   uuid user_id
   uuid org_id
   timestamptz timestamp
   boolean success
   inet ip_address
   text user_agent
   uuid attempt_id
}
class metrics_events {
   uuid user_id
   uuid org_id
   text event_type
   timestamptz event_timestamp
   jsonb metadata
   uuid event_id
}
class mfa_devices {
   uuid user_id
   text type
   text secret
   timestamptz created_at
   boolean enabled
   timestamptz last_used
   uuid device_id
}
class org_login_methods {
   boolean enabled
   uuid org_id
   text method
}
class organizations {
   text org_name
   jsonb org_metadata
   uuid org_id
}
class password_reset_tokens {
   uuid user_id
   timestamptz created_at
   timestamptz expires_at
   timestamptz used_at
   uuid token
}
class permissions {
   uuid org_id
   text name
   text description
   timestamptz created_at
   uuid permission_id
}
class rate_limit_assignments {
   int policy_id
   uuid user_id
   uuid org_id
}
class rate_limit_counters {
   integer count
   timestamptz last_updated
   int policy_id
   uuid user_id
   uuid org_id
   timestamptz window_start
}
class rate_limit_policies {
   text name
   text scope
   integer limit_count
   integer window_sec
   text description
   timestamptz created_at
   integer policy_id
}
class role_inheritance {
   uuid role_id
   uuid parent_role_id
}
class role_permissions {
   uuid role_id
   uuid permission_id
}
class roles {
   uuid org_id
   text name
   text description
   timestamptz created_at
   uuid role_id
}
class scim_clients {
   uuid org_id
   text client_secret
   timestamptz created_at
   timestamptz expires_at
   boolean enabled
   uuid client_id
}
class scim_mappings {
   uuid client_id
   text local_attribute
   text scim_attribute
   uuid mapping_id
}
class scim_tokens {
   uuid client_id
   text token
   timestamptz expires_at
   uuid token_id
}
class sessions {
   uuid user_id
   text refresh_token
   timestamptz created_at
   timestamptz expires_at
   boolean revoked
   inet ip_address
   text user_agent
   uuid session_id
}
class user_organization_info {
   text user_role
   text url_safe_org_name
   text org_role_structure
   text[] inherited_user_roles_plus_current_role
   text[] user_permissions
   text[] additional_roles
   uuid user_id
   uuid org_id
}
class user_roles {
   timestamptz assigned_at
   uuid user_id
   uuid role_id
}
class users {
   text legacy_user_id
   text email
   boolean email_confirmed
   text username
   text first_name
   text last_name
   text picture_url
   jsonb properties
   boolean has_password
   boolean update_password_required
   boolean locked
   boolean enabled
   boolean mfa_enabled
   boolean can_create_orgs
   bigint created_at
   bigint last_active_at
   uuid user_id
}
class waitlist_signups {
   text email
   boolean invited
   timestamptz invited_at
   uuid signup_id
}

api_key_rate_limits  -->  api_keys : api_key_id
api_key_rate_limits  -->  rate_limit_policies : policy_id
api_keys  -->  users : user_id
audit_logs  -->  organizations : org_id
audit_logs  -->  users : user_id
email_verification_tokens  -->  users : user_id
identity_providers  -->  organizations : org_id
impersonation_logs  -->  users : admin_user_id:user_id
impersonation_logs  -->  users : impersonated_user_id:user_id
login_attempts  -->  organizations : org_id
login_attempts  -->  users : user_id
metrics_events  -->  organizations : org_id
metrics_events  -->  users : user_id
mfa_devices  -->  users : user_id
org_login_methods  -->  organizations : org_id
password_reset_tokens  -->  users : user_id
permissions  -->  organizations : org_id
rate_limit_assignments  -->  organizations : org_id
rate_limit_assignments  -->  rate_limit_policies : policy_id
rate_limit_assignments  -->  users : user_id
rate_limit_counters  -->  organizations : org_id
rate_limit_counters  -->  rate_limit_policies : policy_id
rate_limit_counters  -->  users : user_id
role_inheritance  -->  roles : role_id
role_inheritance  -->  roles : parent_role_id:role_id
role_permissions  -->  permissions : permission_id
role_permissions  -->  roles : role_id
roles  -->  organizations : org_id
scim_clients  -->  organizations : org_id
scim_mappings  -->  scim_clients : client_id
scim_tokens  -->  scim_clients : client_id
sessions  -->  users : user_id
user_organization_info  -->  organizations : org_id
user_organization_info  -->  users : user_id
user_roles  -->  roles : role_id
user_roles  -->  users : user_id

```