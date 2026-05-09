/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export type ImportApiKeysRequest = {
    keys: Array<{
        name: string;
        /**
         * Whether the key belongs to a user or org
         */
        owner_type: 'user' | 'org';
        /**
         * Owner user ID or org ID
         */
        owner_id: string;
        /**
         * Pre-computed hash of the API key secret (not the raw secret)
         */
        secret_hash: string;
        /**
         * Permission codes for this key
         */
        permissions?: Array<string>;
        /**
         * Unix timestamp of expiration
         */
        expires_at?: number | null;
        /**
         * Custom metadata
         */
        metadata?: Record<string, any> | null;
    }>;
};

