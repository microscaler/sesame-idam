/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
import type { MfaSetupRequest } from '../models/MfaSetupRequest';
import type { MfaSetupResponse } from '../models/MfaSetupResponse';
import type { MfaVerifyRequest } from '../models/MfaVerifyRequest';
import type { PhoneNumberRequest } from '../models/PhoneNumberRequest';
import type { PhoneVerificationRequest } from '../models/PhoneVerificationRequest';
import type { TokenResponse } from '../models/TokenResponse';
import type { CancelablePromise } from '../core/CancelablePromise';
import { OpenAPI } from '../core/OpenAPI';
import { request as __request } from '../core/request';
export class PasswordSecurityService {
    /**
     * Disable/block user
     * Prevents user from logging in. Platform admin or SaaS with org context.
     *
     * Requires `X-Tenant-ID` header for tenant resolution.
     * @param xTenantId Tenant identifier. Routes the request to the correct tenant context for user lookup and authentication.
     * @param userId
     * @returns any User disabled
     * @throws ApiError
     */
    public static disableUser(
        xTenantId: string,
        userId: string,
    ): CancelablePromise<any> {
        return __request(OpenAPI, {
            method: 'POST',
            url: '/users/{user_id}/disable',
            path: {
                'user_id': userId,
            },
            headers: {
                'X-Tenant-ID': xTenantId,
            },
            errors: {
                404: `Not found`,
            },
        });
    }
    /**
     * Enable/unblock user
     * Re-enables a previously disabled user.
     *
     * Requires `X-Tenant-ID` header for tenant resolution.
     * @param xTenantId Tenant identifier. Routes the request to the correct tenant context for user lookup and authentication.
     * @param userId
     * @returns any User enabled
     * @throws ApiError
     */
    public static enableUser(
        xTenantId: string,
        userId: string,
    ): CancelablePromise<any> {
        return __request(OpenAPI, {
            method: 'POST',
            url: '/users/{user_id}/enable',
            path: {
                'user_id': userId,
            },
            headers: {
                'X-Tenant-ID': xTenantId,
            },
            errors: {
                404: `Not found`,
            },
        });
    }
    /**
     * Clear password (convert to SSO-only)
     * Removes the user's password, converting them to SSO-only authentication.
     * Useful when an organisation enforces SAML SSO and users should no longer use email/password.
     * Platform admin only.
     *
     *
     * Requires `X-Tenant-ID` header for tenant resolution.
     * @param xTenantId Tenant identifier. Routes the request to the correct tenant context for user lookup and authentication.
     * @param userId
     * @returns void
     * @throws ApiError
     */
    public static clearUserPassword(
        xTenantId: string,
        userId: string,
    ): CancelablePromise<void> {
        return __request(OpenAPI, {
            method: 'DELETE',
            url: '/users/{user_id}/password',
            path: {
                'user_id': userId,
            },
            headers: {
                'X-Tenant-ID': xTenantId,
            },
            errors: {
                400: `User has no password to clear`,
                404: `Not found`,
            },
        });
    }
    /**
     * Verify user email
     * Requires `X-Tenant-ID` header for tenant resolution.
     * @param xTenantId Tenant identifier. Routes the request to the correct tenant context for user lookup and authentication.
     * @param userId
     * @returns any Email verified
     * @throws ApiError
     */
    public static verifyUserEmail(
        xTenantId: string,
        userId: string,
    ): CancelablePromise<any> {
        return __request(OpenAPI, {
            method: 'POST',
            url: '/users/{user_id}/email/verify',
            path: {
                'user_id': userId,
            },
            headers: {
                'X-Tenant-ID': xTenantId,
            },
            errors: {
                400: `Invalid token`,
                404: `Not found`,
            },
        });
    }
    /**
     * Resend email confirmation
     * Requires `X-Tenant-ID` header for tenant resolution.
     * @param xTenantId Tenant identifier. Routes the request to the correct tenant context for user lookup and authentication.
     * @param userId
     * @returns any Email sent
     * @throws ApiError
     */
    public static resendEmailConfirmation(
        xTenantId: string,
        userId: string,
    ): CancelablePromise<any> {
        return __request(OpenAPI, {
            method: 'POST',
            url: '/users/{user_id}/resend-email-confirmation',
            path: {
                'user_id': userId,
            },
            headers: {
                'X-Tenant-ID': xTenantId,
            },
            errors: {
                404: `Not found`,
            },
        });
    }
    /**
     * Disable user 2FA
     * Disables all 2FA methods for a user. Platform admin or user themselves.
     *
     * Requires `X-Tenant-ID` header for tenant resolution.
     * @param xTenantId Tenant identifier. Routes the request to the correct tenant context for user lookup and authentication.
     * @param userId
     * @returns any 2FA disabled
     * @throws ApiError
     */
    public static disableUserMfa(
        xTenantId: string,
        userId: string,
    ): CancelablePromise<any> {
        return __request(OpenAPI, {
            method: 'POST',
            url: '/users/{user_id}/mfa/disable',
            path: {
                'user_id': userId,
            },
            headers: {
                'X-Tenant-ID': xTenantId,
            },
            errors: {
                404: `Not found`,
            },
        });
    }
    /**
     * Set up TOTP MFA
     * Generates a TOTP secret QR code for the user to scan with an authenticator app.
     * Requires the user's current password for verification.
     * Returns a provisioning URI that can be rendered as a QR code.
     *
     *
     * Requires `X-Tenant-ID` header for tenant resolution.
     * @param xTenantId Tenant identifier. Routes the request to the correct tenant context for user lookup and authentication.
     * @param userId
     * @param requestBody
     * @returns MfaSetupResponse TOTP secret generated
     * @throws ApiError
     */
    public static setupUserMfaTotp(
        xTenantId: string,
        userId: string,
        requestBody: MfaSetupRequest,
    ): CancelablePromise<MfaSetupResponse> {
        return __request(OpenAPI, {
            method: 'POST',
            url: '/users/{user_id}/mfa/setup',
            path: {
                'user_id': userId,
            },
            headers: {
                'X-Tenant-ID': xTenantId,
            },
            body: requestBody,
            mediaType: 'application/json',
            errors: {
                400: `Invalid password or MFA already configured`,
                404: `Not found`,
            },
        });
    }
    /**
     * Verify MFA code
     * Verifies an MFA code for either:
     * 1. Completing MFA setup (step 2 after setup generated QR code)
     * 2. Completing login (step 2 of password login when MFA is required)
     *
     *
     * Requires `X-Tenant-ID` header for tenant resolution.
     * @param xTenantId Tenant identifier. Routes the request to the correct tenant context for user lookup and authentication.
     * @param userId
     * @param requestBody
     * @returns TokenResponse MFA verified
     * @throws ApiError
     */
    public static verifyUserMfa(
        xTenantId: string,
        userId: string,
        requestBody: MfaVerifyRequest,
    ): CancelablePromise<TokenResponse> {
        return __request(OpenAPI, {
            method: 'POST',
            url: '/users/{user_id}/mfa/verify',
            path: {
                'user_id': userId,
            },
            headers: {
                'X-Tenant-ID': xTenantId,
            },
            body: requestBody,
            mediaType: 'application/json',
            errors: {
                400: `Invalid code`,
                404: `Not found`,
            },
        });
    }
    /**
     * Add phone number for user
     * Adds a phone number to the user's profile and sends an SMS verification code.
     * Phone must be verified before it can be used for authentication.
     *
     *
     * Requires `X-Tenant-ID` header for tenant resolution.
     * @param xTenantId Tenant identifier. Routes the request to the correct tenant context for user lookup and authentication.
     * @param userId
     * @param requestBody
     * @returns any Phone number added, SMS code sent
     * @throws ApiError
     */
    public static setupUserPhone(
        xTenantId: string,
        userId: string,
        requestBody: PhoneNumberRequest,
    ): CancelablePromise<any> {
        return __request(OpenAPI, {
            method: 'POST',
            url: '/users/{user_id}/phone',
            path: {
                'user_id': userId,
            },
            headers: {
                'X-Tenant-ID': xTenantId,
            },
            body: requestBody,
            mediaType: 'application/json',
            errors: {
                400: `Invalid phone number`,
                404: `Not found`,
            },
        });
    }
    /**
     * Verify phone number
     * Verifies the phone number using the SMS OTP code.
     *
     * Requires `X-Tenant-ID` header for tenant resolution.
     * @param xTenantId Tenant identifier. Routes the request to the correct tenant context for user lookup and authentication.
     * @param userId
     * @param requestBody
     * @returns any Phone number verified
     * @throws ApiError
     */
    public static verifyUserPhone(
        xTenantId: string,
        userId: string,
        requestBody: PhoneVerificationRequest,
    ): CancelablePromise<any> {
        return __request(OpenAPI, {
            method: 'POST',
            url: '/users/{user_id}/phone/verify',
            path: {
                'user_id': userId,
            },
            headers: {
                'X-Tenant-ID': xTenantId,
            },
            body: requestBody,
            mediaType: 'application/json',
            errors: {
                400: `Invalid code`,
                404: `Not found`,
            },
        });
    }
}
