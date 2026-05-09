/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
import type { MfaFactor } from './MfaFactor';
/**
 * Complete user entity returned from user lifecycle operations.
 * Contains identity attributes, account status, and migration metadata.
 *
 */
/**
 * Example usage:
 * ```typescript
 * const example: User = {
  "user_id": "31c41c16-c281-44ae-9602-8a047e3bf33d",
  "email": "test@example.com",
  "email_confirmed": true,
  "first_name": "Test",
  "last_name": "User",
  "username": "testuser",
  "picture_url": "https://example.com/avatar.png",
  "properties": {
    "favorite_sport": "basketball",
    "department": "engineering"
  },
  "locked": false,
  "enabled": true,
  "has_password": true,
  "update_password_required": false,
  "mfa_enabled": false,
  "phone_number": "+14155551234",
  "phone_verified": true,
  "mfa_factors": [
    {
      "type": "totp",
      "is_primary": true
    },
    {
      "type": "webauthn",
      "is_primary": false
    }
  ],
  "can_create_orgs": false,
  "created_at": 1625476380,
  "last_active_at": 1625476380
};
 * ```
 */

export type User = {
    /**
     * Unique opaque identifier for this user (not PII)
     */
    user_id: string;
    /**
     * Primary email address (unique, not in URI)
     */
    email: string;
    /**
     * Whether the email address has been verified
     */
    email_confirmed: boolean;
    /**
     * User's first name
     */
    first_name?: string | null;
    /**
     * User's last name
     */
    last_name?: string | null;
    /**
     * Optional human-identifiable username (alphanumeric, underscore, hyphen)
     */
    username?: string | null;
    /**
     * Profile picture URL (e.g. from social login)
     */
    picture_url?: string | null;
    /**
     * Custom key-value metadata attached to the user.
     * Platform admins can set arbitrary properties; SaaS customers limited to org-scoped keys.
     *
     */
    properties?: Record<string, any>;
    /**
     * If true, the user is locked and cannot authenticate.
     * Locked users are typically flagged after repeated failed login attempts.
     *
     */
    locked: boolean;
    /**
     * If true, the user account is active and can authenticate.
     * Disabled users are explicitly blocked (e.g. by admin action).
     *
     */
    enabled: boolean;
    /**
     * Whether the user has a password set.
     * Users with has_password=false can only authenticate via MFA, social login, or SSO.
     *
     */
    has_password: boolean;
    /**
     * If true, the user must change their password at next login.
     * Set when password has expired or was reset by admin.
     *
     */
    update_password_required?: boolean;
    /**
     * Whether any 2FA method is currently enabled for this user
     */
    mfa_enabled?: boolean;
    /**
     * Whether this user can create new organisations (platform admin flag)
     */
    can_create_orgs?: boolean;
    /**
     * Unix epoch timestamp (seconds) when the user was created
     */
    created_at: number;
    /**
     * Unix epoch timestamp of last successful authentication
     */
    last_active_at?: number | null;
    /**
     * Identifier from the previous authentication system.
     * Set during user migration to maintain referential integrity.
     *
     */
    legacy_user_id?: string | null;
    /**
     * Primary phone number in E.164 format (e.g. +14155551234)
     */
    phone_number?: string | null;
    /**
     * Whether the phone number has been verified (via SMS OTP)
     */
    phone_verified?: boolean;
    /**
     * List of 2FA methods currently enabled for this user
     */
    mfa_factors?: Array<MfaFactor>;
};

