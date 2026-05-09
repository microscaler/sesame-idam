/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
import type { ArchivedApiKey } from './ArchivedApiKey';
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

