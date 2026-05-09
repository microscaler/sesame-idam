/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
import type { ApiKey } from './ApiKey';
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
    sort_order?: 'created_at_desc' | 'created_at_asc' | 'name_asc' | 'name_desc' | 'last_used_desc' | 'last_used_asc';
    /**
     * List of filters applied to the query results
     */
    filters_applied?: Array<'active' | 'expired' | 'near_expiry' | 'revoked'>;
};

