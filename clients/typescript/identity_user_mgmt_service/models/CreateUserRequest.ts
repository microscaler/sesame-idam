/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
/**
 * Creates a new user. Idempotent: if an existing user with the same email
 * is found, returns the existing user instead of creating a duplicate.
 * Platform admins may omit `org_id` to create users without org membership.
 *
 */
export type CreateUserRequest = {
    /**
     * Primary email address (must be unique)
     */
    email: string;
    /**
     * If true, the email is considered pre-verified (no confirmation email sent).
     * Useful for platform-admin-created users or migrated accounts.
     *
     */
    email_confirmed?: boolean;
    /**
     * If true, sends a verification email to the user's address.
     * Only meaningful when email_confirmed=false.
     *
     */
    send_email_confirmation?: boolean;
    /**
     * User's first name
     */
    first_name?: string;
    /**
     * User's last name
     */
    last_name?: string;
    /**
     * Optional human-identifiable username
     */
    username?: string;
    /**
     * Initial profile picture URL
     */
    picture_url?: string;
    /**
     * If true, sends a welcome email after user creation
     */
    send_welcome_email?: boolean;
    /**
     * Custom key-value metadata (arbitrary JSON)
     */
    extra_properties?: Record<string, any>;
    /**
     * For SaaS customers: required, must be their own organisation.
     * For platform admins: optional — if omitted, creates a platform-level user.
     * When present, the user is automatically added to this organisation.
     *
     */
    org_id?: string | null;
};

