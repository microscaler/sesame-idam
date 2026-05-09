/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
/**
 * Property examples:
 *  * - `permissions`: `["accounting:invoices:write", "accounting:invoices:read"]`
 * - `expires_in_days`: `90`
 */

export type CreateApiKeyRequest = {
    /**
     * Human-readable name for this API key (e.g., "Production Service")
     */
    name: string;
    /**
     * User ID to scope the key to (omit for org-scoped keys)
     */
    user_id?: string | null;
    /**
     * Organisation ID to scope the key to (omit for user-scoped keys)
     */
    org_id?: string | null;
    /**
     * Permission codes to include in the key. If omitted, includes all permissions
     * for the user/org scope.
     *
     */
    permissions?: Array<string>;
    /**
     * Number of days until the key expires (omit for no expiry)
     */
    expires_in_days?: number | null;
    /**
     * Custom key-value metadata attached to this key
     */
    metadata?: Record<string, any> | null;
};

