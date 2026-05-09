/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export type ApiKeyValidationResponse = {
    /**
     * Whether the API key is valid
     */
    valid: boolean;
    /**
     * API key ID (null if invalid)
     */
    api_key_id?: string | null;
    /**
     * Associated user ID (user-scoped keys only)
     */
    user_id?: string | null;
    /**
     * Associated organisation ID
     */
    org_id?: string | null;
    /**
     * Whether this key is user-scoped or org-scoped
     */
    scope_type?: 'user' | 'org' | null;
    /**
     * Permission codes granted by this key
     */
    permissions?: Array<string> | null;
    /**
     * Expiration timestamp
     */
    expires_at?: number | null;
    /**
     * Whether the key has expired
     */
    is_expired?: boolean;
};

