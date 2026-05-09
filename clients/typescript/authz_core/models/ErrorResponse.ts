/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export type ErrorResponse = {
    /**
     * Machine-readable error code
     */
    error: ErrorResponse.error;
    /**
     * Human-readable error message
     */
    error_description?: string;
};
export namespace ErrorResponse {
    /**
     * Machine-readable error code
     */
    export enum error {
        INVALID_REQUEST = 'invalid_request',
        NOT_FOUND = 'not_found',
        PERMISSION_DENIED = 'permission_denied',
        APPLICATION_NOT_FOUND = 'application_not_found',
        ROLE_NOT_FOUND = 'role_not_found',
        ATTRIBUTE_TOO_LARGE = 'attribute_too_large',
    }
}

