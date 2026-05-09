/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export type ImportApiKeysResponse = {
    /**
     * Number of keys successfully imported
     */
    imported_count?: number;
    /**
     * Number of keys that failed to import
     */
    failed_count?: number;
    errors?: Array<{
        index?: number;
        error?: string;
    }>;
};

