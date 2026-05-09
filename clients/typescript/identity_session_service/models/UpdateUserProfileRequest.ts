/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
/**
 * Partial update of the current user's profile.
 * Only provided fields are modified.
 *
 */
/**
 * Example usage:
 * ```typescript
 * const example: UpdateUserProfileRequest = {
  "first_name": "Updated",
  "last_name": "Name"
};
 * ```
 */

export type UpdateUserProfileRequest = {
    /**
     * Full display name
     */
    name?: string;
    /**
     * Human-identifiable username
     */
    preferred_username?: string;
    /**
     * User's first name
     */
    first_name?: string;
    /**
     * User's last name
     */
    last_name?: string;
    /**
     * Profile picture URL
     */
    picture_url?: string;
};

