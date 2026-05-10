/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export type ApiKey = {
    /**
     * Unique ID of the API key
     */
    api_key_id?: string;
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
     * Whether this key is currently active (not revoked or expired)
     */
    active?: boolean;
    /**
     * Permission codes granted by this key
     */
    permissions?: Array<string>;
    /**
     * Custom metadata attached to this key
     */
    metadata?: Record<string, any> | null;
};

