/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
/**
 * @example {
    "name": "Updated Production Key",
    "permissions": [
        "read",
        "write"
    ]
}
 */
export type UpdateApiKeyRequest = {
    /**
     * Updated name for this API key
     */
    name?: string;
    /**
     * New expiration (in days from now)
     */
    expires_in_days?: number | null;
    /**
     * Updated metadata
     */
    metadata?: Record<string, any> | null;
};

