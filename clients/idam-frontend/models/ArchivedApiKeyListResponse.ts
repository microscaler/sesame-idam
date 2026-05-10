/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
import type { ArchivedApiKey } from './ArchivedApiKey';
/**
 * @example {
    "api_keys": [
        {
            "key_id": "550e8400-e29b-41d4-a716-446655440005",
            "name": "Archived Key",
            "archived_at": "2024-01-10T00:00:00Z",
            "archived_by": "admin@example.com"
        }
    ],
    "total": 1,
    "page": 1,
    "limit": 20
}
 */
export type ArchivedApiKeyListResponse = {
    keys?: Array<ArchivedApiKey>;
    /**
     * Total number of archived API keys matching the query
     */
    total_keys?: number;
    /**
     * Current page number
     */
    current_page?: number;
    /**
     * Number of items per page
     */
    page_size?: number;
    /**
     * Whether additional pages exist
     */
    has_more_results?: boolean;
};

