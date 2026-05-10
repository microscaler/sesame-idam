/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
import type { ApiKey } from './ApiKey';
/**
 * @example {
    "api_keys": [
        {
            "key_id": "550e8400-e29b-41d4-a716-446655440003",
            "name": "Production API Key",
            "key": "sk_live_abc***",
            "permissions": [
                "read",
                "write",
                "delete"
            ],
            "created_at": "2024-01-15T10:30:00Z",
            "expires_at": "2025-01-15T10:30:00Z",
            "last_used_at": "2024-01-16T08:00:00Z"
        },
        {
            "key_id": "550e8400-e29b-41d4-a716-446655440004",
            "name": "Development Key",
            "key": "sk_dev_xyz***",
            "permissions": [
                "read"
            ],
            "created_at": "2024-01-10T00:00:00Z",
            "expires_at": "2024-07-10T00:00:00Z",
            "last_used_at": "2024-01-14T12:00:00Z"
        }
    ],
    "total": 2,
    "page": 1,
    "limit": 20
}
 */
export type ApiKeyListResponse = {
    keys?: Array<ApiKey>;
    /**
     * Total number of active API keys matching the query
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
    /**
     * The sort order applied to the results
     */
    sort_order?: ApiKeyListResponse.sort_order;
    /**
     * List of filters applied to the query results
     */
    filters_applied?: Array<'active' | 'expired' | 'near_expiry' | 'revoked'>;
};
export namespace ApiKeyListResponse {
    /**
     * The sort order applied to the results
     */
    export enum sort_order {
        CREATED_AT_DESC = 'created_at_desc',
        CREATED_AT_ASC = 'created_at_asc',
        NAME_ASC = 'name_asc',
        NAME_DESC = 'name_desc',
        LAST_USED_DESC = 'last_used_desc',
        LAST_USED_ASC = 'last_used_asc',
    }
}

