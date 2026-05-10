/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
import type { ApiKey } from './ApiKey';
/**
 * @example {
    "key_id": "550e8400-e29b-41d4-a716-446655440005",
    "name": "Archived Key",
    "key": "sk_arc_old***",
    "archived_at": "2024-01-10T00:00:00Z",
    "archived_by": "admin@example.com"
}
 */
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

