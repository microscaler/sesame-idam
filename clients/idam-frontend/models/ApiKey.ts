/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
/**
 * @example {
    "key_id": "550e8400-e29b-41d4-a716-446655440003",
    "name": "Updated Production Key",
    "key": "sk_live_abc***",
    "permissions": [
        "read",
        "write"
    ],
    "created_at": "2024-01-15T10:30:00Z",
    "expires_at": "2025-01-15T10:30:00Z",
    "last_used_at": "2024-01-16T08:00:00Z",
    "updated_at": "2024-01-17T10:00:00Z"
}
 */
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

