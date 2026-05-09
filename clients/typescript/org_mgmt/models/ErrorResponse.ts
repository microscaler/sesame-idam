/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
/**
 * Example usage:
 * ```typescript
 * const example: ErrorResponse = {
  "error": "invalid_request",
  "error_description": "Bad request (validation error)"
};
 * ```
 */

export type ErrorResponse = {
    /**
     * Machine-readable error code
     */
    error: 'invalid_request' | 'invalid_credentials' | 'invalid_grant' | 'invalid_code' | 'account_locked' | 'mfa_required' | 'email_not_confirmed' | 'phone_not_verified' | 'duplicate_email' | 'weak_password' | 'rate_limited' | 'invalid_token' | 'not_found' | 'permission_denied' | 'application_not_found' | 'role_not_found' | 'attribute_too_large';
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

