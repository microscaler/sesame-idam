/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
/**
 * Partial update of user attributes. Only provided fields are modified.
 * SaaS customers can only update users within their own organisation.
 *
 */
export type UpdateUserRequest = {
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
     * Profile picture URL
     */
    picture_url?: string;
    /**
     * Force email verification status
     */
    email_confirmed?: boolean;
    /**
     * Platform admin flag controlling org creation rights
     */
    can_create_orgs?: boolean;
    /**
     * Lock or unlock user account
     */
    locked?: boolean;
    /**
     * Send welcome email after update (if email was changed)
     */
    send_welcome_email?: boolean;
    /**
     * Send a magic link to the user's email after update
     */
    send_magic_link?: boolean;
};

