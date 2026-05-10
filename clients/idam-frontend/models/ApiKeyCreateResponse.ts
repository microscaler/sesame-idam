/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export type ApiKeyCreateResponse = {
    /**
     * Unique ID of the created API key
     */
    api_key_id: string;
    /**
     * The raw API key (secret). This is the ONLY time the secret is returned.
     * Store it securely — it cannot be retrieved again.
     *
     */
    api_key: string;
    /**
     * Human-readable name
     */
    name?: string;
    /**
     * User ID if user-scoped
     */
    user_id?: string | null;
    /**
     * Organisation ID if org-scoped
     */
    org_id?: string | null;
    /**
     * Unix timestamp of creation
     */
    created_at?: number;
    /**
     * Unix timestamp of expiration (null = no expiry)
     */
    expires_at?: number | null;
    /**
     * Permission codes granted by this key
     */
    permissions?: Array<string>;
};

