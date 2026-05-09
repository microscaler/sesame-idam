/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
/**
 * OIDC-compliant user profile. This is the shape returned by:
 * - GET /api/v1/identity/users/me (current user profile)
 * - GET /oauth/userinfo (OIDC userinfo endpoint)
 * - LoginResponse.user (enriched with active org context)
 *
 * Combines standard OIDC claims (sub, email_verified, name, preferred_username)
 * with identity-specific fields (user_id, first_name, last_name) and session context
 * (org_id, org_name, user_role, user_permissions).
 *
 */
/**
 * Example usage:
 * ```typescript
 * const example: UserProfile = {
  "sub": "31c41c16-c281-44ae-9602-8a047e3bf33d",
  "email": "test@example.com",
  "email_verified": true,
  "name": "Test User",
  "preferred_username": "testuser",
  "first_name": "Test",
  "last_name": "User",
  "username": "testuser",
  "picture_url": "https://example.com/avatar.png",
  "phone_number": "+14155551234",
  "phone_verified": true,
  "org_id": "1189c444-8a2d-4c41-8b4b-ae43ce79a492",
  "org_name": "Acme Inc",
  "user_role": "Admin",
  "user_permissions": [
    "accounting:invoices:write",
    "accounting:invoices:read"
  ],
  "updated_at": "2025-01-15T10:30:00Z"
};
 * ```
 */

export type UserProfile = {
    /**
     * Subject — the user's opaque identifier (not PII)
     */
    sub?: string;
    /**
     * Primary email address
     */
    email?: string | null;
    /**
     * Whether the email address has been verified
     */
    email_verified?: boolean;
    /**
     * Full display name (first_name + " " + last_name)
     */
    name?: string | null;
    /**
     * Human-identifiable username (alias for username)
     */
    preferred_username?: string | null;
    /**
     * Opaque user identifier (alias for sub)
     */
    user_id?: string | null;
    /**
     * User's first name
     */
    first_name?: string | null;
    /**
     * User's last name
     */
    last_name?: string | null;
    /**
     * Alias for preferred_username
     */
    username?: string | null;
    /**
     * Profile picture URL
     */
    picture_url?: string | null;
    /**
     * Custom key-value metadata (user-level, not org-scoped)
     */
    properties?: Record<string, any>;
    /**
     * Primary phone number in E.164 format (e.g. +14155551234)
     */
    phone_number?: string | null;
    /**
     * Whether the phone number has been verified (via SMS OTP)
     */
    phone_verified?: boolean;
    /**
     * Active organisation context (set by JWT enrichment)
     */
    org_id?: string | null;
    /**
     * Name of the active organisation
     */
    org_name?: string | null;
    /**
     * Primary role in the active organisation
     */
    user_role?: string | null;
    /**
     * Permission codes for the active organisation.
     * Uses dot-notation format (e.g. "accounting:invoices:read")
     *
     */
    user_permissions?: Array<string> | null;
    /**
     * Last profile update timestamp
     */
    updated_at?: string | null;
};

