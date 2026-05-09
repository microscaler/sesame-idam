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
    /**
     * Additional guidance for resolving the error
     */
    hint?: string;
    /**
     * Retry-After seconds (only for rate_limited)
     */
    retry_after?: number;
};
export namespace ErrorResponse {
    /**
     * Machine-readable error code
     */
    export enum error {
        INVALID_REQUEST = 'invalid_request',
        INVALID_CREDENTIALS = 'invalid_credentials',
        INVALID_GRANT = 'invalid_grant',
        INVALID_CODE = 'invalid_code',
        ACCOUNT_LOCKED = 'account_locked',
        MFA_REQUIRED = 'mfa_required',
        EMAIL_NOT_CONFIRMED = 'email_not_confirmed',
        PHONE_NOT_VERIFIED = 'phone_not_verified',
        DUPLICATE_EMAIL = 'duplicate_email',
        WEAK_PASSWORD = 'weak_password',
        RATE_LIMITED = 'rate_limited',
        INVALID_TOKEN = 'invalid_token',
        NOT_FOUND = 'not_found',
        PERMISSION_DENIED = 'permission_denied',
        APPLICATION_NOT_FOUND = 'application_not_found',
        ROLE_NOT_FOUND = 'role_not_found',
        ATTRIBUTE_TOO_LARGE = 'attribute_too_large',
    }
}

