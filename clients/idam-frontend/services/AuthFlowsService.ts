/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
import type { DualOTPCompleteResponse } from '../models/DualOTPCompleteResponse';
import type { DualOTPPartialResponse } from '../models/DualOTPPartialResponse';
import type { DualOTPRequest } from '../models/DualOTPRequest';
import type { DualOTPResponse } from '../models/DualOTPResponse';
import type { DualOTPVerifyRequest } from '../models/DualOTPVerifyRequest';
import type { EmailOTPRequest } from '../models/EmailOTPRequest';
import type { EmailOTPVerifyRequest } from '../models/EmailOTPVerifyRequest';
import type { LoginRequest } from '../models/LoginRequest';
import type { MfaRequiredResponse } from '../models/MfaRequiredResponse';
import type { PhoneOTPRequest } from '../models/PhoneOTPRequest';
import type { PhoneOTPVerifyRequest } from '../models/PhoneOTPVerifyRequest';
import type { RegisterRequest } from '../models/RegisterRequest';
import type { TokenResponse } from '../models/TokenResponse';
import type { CancelablePromise } from '../core/CancelablePromise';
import { OpenAPI } from '../core/OpenAPI';
import { request as __request } from '../core/request';
export class AuthFlowsService {
    /**
     * Login with password
     * Authenticate with email+password. Returns access token and refresh token.
     * If MFA is enabled on the account, returns 202 with MFA verification flow.
     *
     *
     * Requires `X-Tenant-ID` header for tenant resolution.
     * @param xTenantId Tenant identifier. Routes the request to the correct tenant context for user lookup and authentication.
     * @param requestBody
     * @returns TokenResponse Login successful
     * @returns MfaRequiredResponse Login successful
     * @throws ApiError
     */
    public static authLogin(
        xTenantId: string,
        requestBody: LoginRequest,
    ): CancelablePromise<TokenResponse | MfaRequiredResponse> {
        return __request(OpenAPI, {
            method: 'POST',
            url: '/login',
            headers: {
                'X-Tenant-ID': xTenantId,
            },
            body: requestBody,
            mediaType: 'application/json',
            errors: {
                400: `Bad request (validation error)`,
                401: `Invalid credentials`,
            },
        });
    }
    /**
     * Send email OTP
     * Sends a time-limited OTP code to the user's email address.
     * Used for passwordless email login and dual OTP verification.
     *
     *
     * Requires `X-Tenant-ID` header for tenant resolution.
     * @param xTenantId Tenant identifier. Routes the request to the correct tenant context for user lookup and authentication.
     * @param requestBody
     * @returns any OTP sent successfully
     * @throws ApiError
     */
    public static loginEmailOtp(
        xTenantId: string,
        requestBody: EmailOTPRequest,
    ): CancelablePromise<{
        success?: boolean;
        message?: string;
    }> {
        return __request(OpenAPI, {
            method: 'POST',
            url: '/login/email-otp',
            headers: {
                'X-Tenant-ID': xTenantId,
            },
            body: requestBody,
            mediaType: 'application/json',
            errors: {
                400: `Bad request (invalid email, account not found)`,
            },
        });
    }
    /**
     * Verify email OTP and complete login
     * Verifies the OTP code sent to the user's email. On success, returns
     * access token and refresh token. This is step 2 of the email-only OTP flow.
     *
     *
     * Requires `X-Tenant-ID` header for tenant resolution.
     * @param xTenantId Tenant identifier. Required for resolving the user to the correct tenant context.
     * @param requestBody
     * @returns TokenResponse Email verified, tokens issued
     * @throws ApiError
     */
    public static verifyEmailOtp(
        xTenantId: string,
        requestBody: EmailOTPVerifyRequest,
    ): CancelablePromise<TokenResponse> {
        return __request(OpenAPI, {
            method: 'POST',
            url: '/verify/email-otp',
            headers: {
                'X-Tenant-ID': xTenantId,
            },
            body: requestBody,
            mediaType: 'application/json',
            errors: {
                400: `Invalid or expired code`,
            },
        });
    }
    /**
     * Send phone SMS OTP
     * Sends a 6-digit SMS OTP code to the user's phone number.
     * Used for passwordless phone login and dual OTP verification.
     *
     *
     * Requires `X-Tenant-ID` header for tenant resolution.
     * @param xTenantId Tenant identifier. Routes the request to the correct tenant context for user lookup and authentication.
     * @param requestBody
     * @returns any OTP sent successfully
     * @throws ApiError
     */
    public static loginPhoneOtp(
        xTenantId: string,
        requestBody: PhoneOTPRequest,
    ): CancelablePromise<{
        success?: boolean;
        message?: string;
    }> {
        return __request(OpenAPI, {
            method: 'POST',
            url: '/login/phone-otp',
            headers: {
                'X-Tenant-ID': xTenantId,
            },
            body: requestBody,
            mediaType: 'application/json',
            errors: {
                400: `Bad request (invalid phone, SMS provider error)`,
            },
        });
    }
    /**
     * Verify phone SMS OTP and complete login
     * Verifies the 6-digit SMS OTP code. On success, returns access token
     * and refresh token. Sets phone_verified=true on the user's profile.
     * This is step 2 of the phone-only OTP flow.
     *
     *
     * Requires `X-Tenant-ID` header for tenant resolution.
     * @param xTenantId Tenant identifier. Required for resolving the user to the correct tenant context.
     * @param requestBody
     * @returns TokenResponse Phone verified, tokens issued
     * @throws ApiError
     */
    public static verifyPhoneOtp(
        xTenantId: string,
        requestBody: PhoneOTPVerifyRequest,
    ): CancelablePromise<TokenResponse> {
        return __request(OpenAPI, {
            method: 'POST',
            url: '/verify/phone-otp',
            headers: {
                'X-Tenant-ID': xTenantId,
            },
            body: requestBody,
            mediaType: 'application/json',
            errors: {
                400: `Invalid or expired code`,
            },
        });
    }
    /**
     * Send OTPs to both email and phone simultaneously
     * Step 1 of dual OTP verification. Sends OTP codes to both the user's
     * email and phone number. User must verify both codes before login completes.
     * Used when a user is signing up or logging in with both email and phone provided.
     *
     *
     * Requires `X-Tenant-ID` header for tenant resolution.
     * @param xTenantId Tenant identifier. Routes the request to the correct tenant context for user lookup and authentication.
     * @param requestBody
     * @returns DualOTPResponse OTPs sent to both channels
     * @throws ApiError
     */
    public static loginDualOtp(
        xTenantId: string,
        requestBody: DualOTPRequest,
    ): CancelablePromise<DualOTPResponse> {
        return __request(OpenAPI, {
            method: 'POST',
            url: '/login/dual-otp',
            headers: {
                'X-Tenant-ID': xTenantId,
            },
            body: requestBody,
            mediaType: 'application/json',
            errors: {
                400: `Bad request (missing email or phone, user not found)`,
            },
        });
    }
    /**
     * Verify dual OTP codes and complete login
     * Step 2 of dual OTP verification. Accepts one or both OTP codes.
     * User can verify email and phone in any order. On both verified, returns tokens.
     *
     *
     * Requires `X-Tenant-ID` header for tenant resolution.
     * @param xTenantId Tenant identifier. Required for resolving the user to the correct tenant context.
     * @param requestBody
     * @returns DualOTPCompleteResponse Both codes verified, tokens issued
     * @returns DualOTPPartialResponse Dual OTP verification successful
     * @throws ApiError
     */
    public static verifyDualOtp(
        xTenantId: string,
        requestBody: DualOTPVerifyRequest,
    ): CancelablePromise<DualOTPCompleteResponse | DualOTPPartialResponse> {
        return __request(OpenAPI, {
            method: 'POST',
            url: '/verify/dual-otp',
            headers: {
                'X-Tenant-ID': xTenantId,
            },
            body: requestBody,
            mediaType: 'application/json',
            errors: {
                400: `Invalid or expired codes`,
            },
        });
    }
    /**
     * Register new user with email and password
     * Creates a new user account with email+password.
     * Returns access token immediately if email auto-verified, otherwise returns
     * pending-verification state requiring email confirmation.
     *
     *
     * Requires `X-Tenant-ID` header for tenant resolution.
     * @param xTenantId Tenant identifier. Routes the request to the correct tenant context for user lookup and authentication.
     * @param requestBody
     * @returns TokenResponse User created and logged in
     * @throws ApiError
     */
    public static authRegister(
        xTenantId: string,
        requestBody: RegisterRequest,
    ): CancelablePromise<TokenResponse> {
        return __request(OpenAPI, {
            method: 'POST',
            url: '/register',
            headers: {
                'X-Tenant-ID': xTenantId,
            },
            body: requestBody,
            mediaType: 'application/json',
            errors: {
                400: `Validation error (duplicate email, weak password)`,
            },
        });
    }
}
