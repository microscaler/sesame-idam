/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
import type { ForgotPasswordRequest } from '../models/ForgotPasswordRequest';
import type { ResetPasswordRequest } from '../models/ResetPasswordRequest';
import type { CancelablePromise } from '../core/CancelablePromise';
import { OpenAPI } from '../core/OpenAPI';
import { request as __request } from '../core/request';
export class PasswordResetService {
    /**
     * Request password reset email
     * Sends a password reset email with a time-limited token to the given email.
     *
     *
     * Requires `X-Tenant-ID` header for tenant resolution.
     * @param xTenantId Tenant identifier. Routes the request to the correct tenant context for user lookup and authentication.
     * @param requestBody
     * @returns any Reset email sent (returns success regardless of email existence)
     * @throws ApiError
     */
    public static authForgotPassword(
        xTenantId: string,
        requestBody?: ForgotPasswordRequest,
    ): CancelablePromise<{
        /**
         * Whether the request was accepted
         * @example true
         */
        success: boolean;
        /**
         * Human-readable message for the user
         * @example If the email is registered, a reset link has been sent
         */
        message: string;
        /**
         * Token expiry time in minutes (e.g., 15)
         * @example 15
         */
        expires_in?: number;
        /**
         * Type of token issued (helps clients handle display)
         * @example reset
         */
        token_type?: string;
    }> {
        return __request(OpenAPI, {
            method: 'POST',
            url: '/forgot-password',
            headers: {
                'X-Tenant-ID': xTenantId,
            },
            body: requestBody,
            mediaType: 'application/json',
            errors: {
                400: `Bad request`,
            },
        });
    }
    /**
     * Confirm password reset with token
     * Validates the reset token from the password reset email and sets a new password.
     *
     *
     * Requires `X-Tenant-ID` header for tenant resolution.
     * @param xTenantId Tenant identifier. Routes the request to the correct tenant context for user lookup and authentication.
     * @param requestBody
     * @returns any Password reset successful
     * @throws ApiError
     */
    public static authResetPassword(
        xTenantId: string,
        requestBody?: ResetPasswordRequest,
    ): CancelablePromise<{
        /**
         * @example true
         */
        success?: boolean;
        /**
         * @example Password has been reset successfully
         */
        message?: string;
    }> {
        return __request(OpenAPI, {
            method: 'POST',
            url: '/reset-password',
            headers: {
                'X-Tenant-ID': xTenantId,
            },
            body: requestBody,
            mediaType: 'application/json',
            errors: {
                400: `Invalid or expired reset token, weak new password`,
            },
        });
    }
}
