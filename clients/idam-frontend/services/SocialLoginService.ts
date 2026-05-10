/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
import type { LinkSocialAccountRequest } from '../models/LinkSocialAccountRequest';
import type { LinkSocialAccountResponse } from '../models/LinkSocialAccountResponse';
import type { OAuthTokenResponse } from '../models/OAuthTokenResponse';
import type { TokenListResponse } from '../models/TokenListResponse';
import type { CancelablePromise } from '../core/CancelablePromise';
import { OpenAPI } from '../core/OpenAPI';
import { request as __request } from '../core/request';
export class SocialLoginService {
    /**
     * Initiate OAuth login with provider
     * Redirects the user to the OAuth provider's login page.
     * Providers: google, github, linkedin, facebook, apple, microsoft, or custom.
     * For GitHub specifically: used by PriceWhisperer platform login flow.
     *
     *
     * Requires `X-Tenant-ID` header for tenant resolution.
     * @param xTenantId Tenant identifier. Routes the request to the correct tenant context for user lookup and authentication.
     * @param provider OAuth provider name (e.g., github, google)
     * @param redirectUri Where to redirect after OAuth completion
     * @param scope Additional OAuth scopes (e.g., "user:email" for GitHub)
     * @returns void
     * @throws ApiError
     */
    public static socialLogin(
        xTenantId: string,
        provider: string,
        redirectUri: string,
        scope?: string,
    ): CancelablePromise<void> {
        return __request(OpenAPI, {
            method: 'GET',
            url: '/social/{provider}/login',
            path: {
                'provider': provider,
            },
            headers: {
                'X-Tenant-ID': xTenantId,
            },
            query: {
                'redirect_uri': redirectUri,
                'scope': scope,
            },
            errors: {
                302: `Redirect to OAuth provider authorization page`,
                400: `Unsupported provider or missing redirect_uri`,
            },
        });
    }
    /**
     * Link social account to user
     * Redirects user to OAuth provider to link an additional social account.
     * Requires authentication (user must be logged in).
     *
     *
     * Requires `X-Tenant-ID` header for tenant resolution.
     * @param xTenantId Tenant identifier. Routes the request to the correct tenant context for user lookup and authentication.
     * @param userId
     * @param requestBody
     * @returns LinkSocialAccountResponse OK - Redirect URL generated successfully
     * @throws ApiError
     */
    public static linkSocialAccount(
        xTenantId: string,
        userId: string,
        requestBody: LinkSocialAccountRequest,
    ): CancelablePromise<LinkSocialAccountResponse> {
        return __request(OpenAPI, {
            method: 'POST',
            url: '/users/{user_id}/social/link',
            path: {
                'user_id': userId,
            },
            headers: {
                'X-Tenant-ID': xTenantId,
            },
            body: requestBody,
            mediaType: 'application/json',
            errors: {
                400: `Bad Request - Invalid provider or user_id`,
                401: `Unauthorized - Missing or invalid authentication`,
                404: `Not Found - User not found`,
            },
        });
    }
    /**
     * Fetch user's OAuth tokens from providers
     * Requires `X-Tenant-ID` header for tenant resolution.
     * @param xTenantId Tenant identifier. Routes the request to the correct tenant context for user lookup and authentication.
     * @param userId
     * @returns TokenListResponse
     * @throws ApiError
     */
    public static fetchUserOauthTokens(
        xTenantId: string,
        userId: string,
    ): CancelablePromise<TokenListResponse> {
        return __request(OpenAPI, {
            method: 'GET',
            url: '/users/{user_id}/social/tokens',
            path: {
                'user_id': userId,
            },
            headers: {
                'X-Tenant-ID': xTenantId,
            },
        });
    }
    /**
     * Fetch fresh token from provider
     * Refreshes an expired OAuth token for the given provider.
     *
     * Requires `X-Tenant-ID` header for tenant resolution.
     * @param xTenantId Tenant identifier. Routes the request to the correct tenant context for user lookup and authentication.
     * @param userId
     * @param provider
     * @returns OAuthTokenResponse
     * @throws ApiError
     */
    public static fetchFreshOauthToken(
        xTenantId: string,
        userId: string,
        provider: string,
    ): CancelablePromise<OAuthTokenResponse> {
        return __request(OpenAPI, {
            method: 'GET',
            url: '/users/{user_id}/social/tokens/{provider}/refresh',
            path: {
                'user_id': userId,
                'provider': provider,
            },
            headers: {
                'X-Tenant-ID': xTenantId,
            },
        });
    }
}
