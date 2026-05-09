/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
import type { SocialCallbackRequest } from '../models/SocialCallbackRequest';
import type { SocialLoginResponse } from '../models/SocialLoginResponse';
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
     * Exchange OAuth provider callback for tokens
     * Step 2 of social login. Receives the provider's authorization code
     * (from OAuth redirect), exchanges it for tokens, and completes login.
     * For GitHub: exchanges the GitHub code for user profile + access tokens.
     *
     *
     * Requires `X-Tenant-ID` header for tenant resolution.
     * @param xTenantId Tenant identifier. Required for resolving the user to the correct tenant context.
     * @param provider OAuth provider name
     * @param requestBody
     * @returns SocialLoginResponse OAuth flow complete, tokens issued
     * @throws ApiError
     */
    public static socialCallback(
        xTenantId: string,
        provider: string,
        requestBody: SocialCallbackRequest,
    ): CancelablePromise<SocialLoginResponse> {
        return __request(OpenAPI, {
            method: 'POST',
            url: '/social/{provider}/callback',
            path: {
                'provider': provider,
            },
            headers: {
                'X-Tenant-ID': xTenantId,
            },
            body: requestBody,
            mediaType: 'application/json',
            errors: {
                400: `Invalid code, state mismatch, or provider error`,
            },
        });
    }
}
