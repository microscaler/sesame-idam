# Sesame-IDAM Entity Model

> Canonical entity model with relationships.
> Date: 2026-05-02 (updated)

---

## Core Entity Class Diagram

```mermaid
classDiagram
    class User {
        UUID id PK
        text email (unique)
        boolean email_verified
        text phone_number
        boolean phone_verified
        text user_type ('customer' | 'platform')
        text display_name
        text profile_image_url
        jsonb properties
        boolean has_password
        text password_hash
        boolean locked
        boolean enabled
        timestamptz created_at
        timestamptz updated_at
        timestamptz last_active_at
        timestamptz deleted_at (soft delete)
    }

    class Organization {
        UUID id PK
        text platform (app domain)
        text name
        text slug (unique per platform)
        jsonb metadata
        jsonb settings
        timestamptz created_at
        timestamptz updated_at
        timestamptz deleted_at
    }

    class OrganizationMember {
        UUID organization_id FK
        UUID user_id FK
        UUID role_id FK
        timestamptz joined_at
        timestamptz invited_at
        UUID invited_by
        text status ('invited' | 'active' | 'removed')
    }

    class Application {
        UUID id PK
        text platform_domain (unique)
        text name
        boolean is_active
        timestamptz created_at
        timestamptz updated_at
    }

    class Role {
        UUID id PK
        UUID application_id FK
        UUID organization_id FK (nullable)
        text name
        text display_name
        text description
        boolean is_system
        UUID parent_role_id FK (self-ref)
        timestamptz created_at
        timestamptz updated_at
    }

    class Permission {
        UUID id PK
        UUID application_id FK
        text name (e.g. "invoices:write")
        text description
        timestamptz created_at
    }

    class RolePermission {
        UUID role_id FK
        UUID permission_id FK
    }

    class UserRole {
        UUID user_id FK
        UUID organization_id FK
        UUID role_id FK
        UUID granted_by
        timestamptz granted_at
    }

    class Session {
        UUID id PK
        UUID user_id FK
        UUID application_id FK
        text session_token (hashed)
        text refresh_token (hashed)
        INET ip_address
        text user_agent
        timestamptz created_at
        timestamptz expires_at
        boolean revoked
        timestamptz last_used_at
    }

    class APIKey {
        UUID id PK
        UUID application_id FK
        text key_hash
        text key_prefix
        text description
        text[] permissions
        timestamptz expires_at
        boolean revoked
        UUID created_by
        timestamptz created_at
        timestamptz last_used_at
    }

    class MFADevice {
        UUID id PK
        UUID user_id FK
        text type ('totp' | 'webauthn' | 'sms')
        text secret (encrypted)
        boolean is_active
        text label
        timestamptz created_at
        timestamptz last_used_at
    }

    class AuditLog {
        UUID id PK
        UUID user_id
        UUID organization_id
        UUID application_id
        text action
        text resource_type
        text resource_id
        jsonb metadata
        INET ip_address
        text user_agent
        timestamptz created_at
    }

    class WebhookEndpoint {
        UUID id PK
        UUID application_id FK
        text url
        text secret (HMAC key)
        text[] events
        boolean is_active
        timestamptz created_at
        timestamptz updated_at
    }

    class WebhookDelivery {
        UUID id PK
        UUID webhook_endpoint_id FK
        text event_type
        jsonb payload
        text status ('pending' | 'success' | 'failed')
        integer attempts
        timestamptz last_attempt_at
        timestamptz next_retry_at
        integer response_status
        text response_body
        timestamptz created_at
    }

    Organization "1" *-- "0..*" OrganizationMember
    User "1" *-- "0..*" OrganizationMember
    Organization "1" *-- "0..*" Role
    Organization "1" *-- "0..*" Permission
    Role "1" *-- "0..*"" RolePermission
    Permission "1" *-- "0..*"" RolePermission
    User "1" *-- "0..*"" UserRole
    Role "1" *-- "0..*"" UserRole
    User "1" *-- "0..*"" Session
    Application "1" *-- "0..*"" Session
    User "1" *-- "0..*"" APIKey
    Application "1" *-- "0..*"" APIKey
    User "1" *-- "0..*"" MFADevice
    User "1" *-- "0..*"" AuditLog
    Application "1" *-- "0..*"" WebhookEndpoint
    WebhookEndpoint "1" *-- "0..*"" WebhookDelivery
    Role "1" *-- "0..*"" Role : parent_role_id
```
