/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
import type { ApiKey } from './ApiKey';
export type ArchivedApiKey = (ApiKey & {
    /**
     * Unix timestamp when the key was revoked
     */
    revoked_at?: number | null;
    /**
     * User ID of the user who revoked the key
     */
    revoked_by_user_id?: string | null;
    /**
     * Reason for revocation
     */
    reason?: string | null;
    /**
     * Why the key is archived (expired, revoked, etc.)
     */
    archived_reason?: string | null;
});

