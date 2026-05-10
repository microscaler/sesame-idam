/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
/**
 * @example {
    "key_id": "550e8400-e29b-41d4-a716-446655440003",
    "total_requests": 15234,
    "requests_last_24h": 342,
    "requests_last_7d": 2891,
    "requests_last_30d": 11456
}
 */
export type ApiKeyUsageResponse = {
    /**
     * The date for which usage is reported
     */
    date?: string;
    /**
     * Total number of API key validation calls on this date
     */
    total_validations?: number;
};

