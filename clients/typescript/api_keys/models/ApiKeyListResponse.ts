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

