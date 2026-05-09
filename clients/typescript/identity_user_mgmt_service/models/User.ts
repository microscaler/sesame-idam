/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
/**
 * Complete user entity returned from user lifecycle operations.
 * Contains identity attributes, account status, and migration metadata.
 *
 */
export type User = {
    /**
     * Unique opaque identifier for this user (not PII)
     */
    user_id?: string;
    /**
     * Primary email address (unique, not in URI)
     */
    email?: string;
    /**
     * Whether the email address has been verified
     */
    email_confirmed?: boolean;
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
    locked?: boolean;
    /**
     * If true, the user account is active and can authenticate.
     * Disabled users are explicitly blocked (e.g. by admin action).
     *
     */
    enabled?: boolean;
    /**
     * Whether the user has a password set.
     *
     */
    has_password?: boolean;
};

