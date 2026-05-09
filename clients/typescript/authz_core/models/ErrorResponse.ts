/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export type ErrorResponse = {
    /**
     * Machine-readable error code
     */
    error: 'invalid_request' | 'not_found' | 'permission_denied' | 'application_not_found' | 'role_not_found' | 'attribute_too_large';
    /**
     * Human-readable error message
     */
    error_description?: string;
};

