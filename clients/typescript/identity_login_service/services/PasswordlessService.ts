/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
import type { MagicLinkRequest } from '../models/MagicLinkRequest';
import type { MagicLinkResponse } from '../models/MagicLinkResponse';
import type { MagicLinkVerifyRequest } from '../models/MagicLinkVerifyRequest';
import type { SmsMagicLinkRequest } from '../models/SmsMagicLinkRequest';
import type { SmsMagicLinkResponse } from '../models/SmsMagicLinkResponse';
import type { SmsMagicLinkVerifyRequest } from '../models/SmsMagicLinkVerifyRequest';
import type { TokenResponse } from '../models/TokenResponse';
import type { CancelablePromise } from '../core/CancelablePromise';
import { OpenAPI } from '../core/OpenAPI';
import { request as __request } from '../core/request';
export class PasswordlessService {
    /**
     * Send magic link for passwordless login
     * Sends a time-limited magic link to the user email. The user clicks the link to complete login without a password.
     *
     * Requires `X-Tenant-ID` header for tenant resolution.
     * @param xTenantId Tenant identifier. Routes the request to the correct tenant context for user lookup and authentication.
     * @param requestBody
     * @returns MagicLinkResponse Magic link sent
     * @throws ApiError
     */
    public static magicLinkSend(
        xTenantId: string,
        requestBody: MagicLinkRequest,
    ): CancelablePromise<MagicLinkResponse> {
        return __request(OpenAPI, {
            method: 'POST',
            url: '/login/magic-link',
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
     * Verify magic link token and complete login
     * Verifies the magic link token. On success, returns access token and refresh token.
     *
     * Requires `X-Tenant-ID` header for tenant resolution.
     * @param xTenantId Tenant identifier. Routes the request to the correct tenant context for user lookup and authentication.
     * @param requestBody
     * @returns TokenResponse Magic link verified, tokens issued
     * @throws ApiError
     */
    public static magicLinkVerify(
        xTenantId: string,
        requestBody: MagicLinkVerifyRequest,
    ): CancelablePromise<TokenResponse> {
        return __request(OpenAPI, {
            method: 'POST',
            url: '/login/magic-link/verify',
            headers: {
                'X-Tenant-ID': xTenantId,
            },
            body: requestBody,
            mediaType: 'application/json',
            errors: {
                400: `Invalid or expired token`,
            },
        });
    }
    /**
     * Send SMS magic link for passwordless login
     * Sends a time-limited magic link via SMS to the user phone.
     *
     * Requires `X-Tenant-ID` header for tenant resolution.
     * @param xTenantId Tenant identifier. Routes the request to the correct tenant context for user lookup and authentication.
     * @param requestBody
     * @returns SmsMagicLinkResponse SMS magic link sent
     * @throws ApiError
     */
    public static smsMagicLinkSend(
        xTenantId: string,
        requestBody: SmsMagicLinkRequest,
    ): CancelablePromise<SmsMagicLinkResponse> {
        return __request(OpenAPI, {
            method: 'POST',
            url: '/login/phone-magic-link',
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
     * Verify SMS magic link token and complete login
     * Verifies the SMS magic link token. On success, returns access token and refresh token.
     *
     * Requires `X-Tenant-ID` header for tenant resolution.
     * @param xTenantId Tenant identifier. Routes the request to the correct tenant context for user lookup and authentication.
     * @param requestBody
     * @returns TokenResponse SMS magic link verified, tokens issued
     * @throws ApiError
     */
    public static smsMagicLinkVerify(
        xTenantId: string,
        requestBody: SmsMagicLinkVerifyRequest,
    ): CancelablePromise<TokenResponse> {
        return __request(OpenAPI, {
            method: 'POST',
            url: '/login/phone-magic-link/verify',
            headers: {
                'X-Tenant-ID': xTenantId,
            },
            body: requestBody,
            mediaType: 'application/json',
            errors: {
                400: `Invalid or expired token`,
            },
        });
    }
}
