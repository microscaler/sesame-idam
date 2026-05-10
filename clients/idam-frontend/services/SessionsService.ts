/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
import type { LogoutRequest } from '../models/LogoutRequest';
import type { OAuthLogoutRequest } from '../models/OAuthLogoutRequest';
import type { RefreshRequest } from '../models/RefreshRequest';
import type { TokenRequest } from '../models/TokenRequest';
import type { TokenResponse } from '../models/TokenResponse';
import type { UserProfile } from '../models/UserProfile';
import type { CancelablePromise } from '../core/CancelablePromise';
import { OpenAPI } from '../core/OpenAPI';
import { request as __request } from '../core/request';
export class SessionsService {
    /**
     * Token endpoint (refresh, client_credentials, token_exchange RFC 8693)
     * OAuth2 token endpoint supporting:
     * - refresh_token grant: rotate refresh token, issue new access token
     * - client_credentials grant: M2M token for service accounts
     * - urn:ietf:params:oauth:grant-type:token-exchange: cross-token exchange
     *
     *
     * Requires `X-Tenant-ID` header for tenant resolution.
     * @param xTenantId Tenant identifier. Routes the request to the correct tenant context for user lookup and authentication.
     * @param requestBody
     * @returns TokenResponse Token issued or refreshed
     * @throws ApiError
     */
    public static authToken(
        xTenantId: string,
        requestBody: TokenRequest,
    ): CancelablePromise<TokenResponse> {
        return __request(OpenAPI, {
            method: 'POST',
            url: '/token',
            headers: {
                'X-Tenant-ID': xTenantId,
            },
            body: requestBody,
            mediaType: 'application/json',
            errors: {
                400: `Invalid grant type or parameters`,
                401: `Invalid or expired refresh token`,
            },
        });
    }
    /**
     * Logout (revoke refresh token)
     * Revokes the current refresh token and ends the user's session.
     * Requires BearerAuth header with a valid access token.
     *
     *
     * Requires `X-Tenant-ID` header for tenant resolution.
     * @param xTenantId Tenant identifier. Routes the request to the correct tenant context for user lookup and authentication.
     * @param requestBody
     * @returns void
     * @throws ApiError
     */
    public static authLogout(
        xTenantId: string,
        requestBody?: LogoutRequest,
    ): CancelablePromise<void> {
        return __request(OpenAPI, {
            method: 'POST',
            url: '/logout',
            headers: {
                'X-Tenant-ID': xTenantId,
            },
            body: requestBody,
            mediaType: 'application/json',
            errors: {
                401: `Unauthorized (invalid or missing token)`,
            },
        });
    }
    /**
     * OAuth2 authorization endpoint
     * Redirects user to the Sesame login page. Compatible with `code` response type.
     * After successful login, redirects to `redirect_uri` with `code` parameter.
     * Supports PKCE for public clients.
     *
     *
     * Requires `X-Tenant-ID` header for tenant resolution.
     * @param xTenantId Tenant identifier. Routes the request to the correct tenant context for user lookup and authentication.
     * @param clientId OAuth2 client ID
     * @param responseType Must be `code` for authorization code flow
     * @param redirectUri Registered redirect URI
     * @param state CSRF protection state parameter
     * @param scope Requested scopes (openid, email, profile, etc.)
     * @param codeChallenge PKCE code challenge (S256)
     * @param codeChallengeMethod PKCE method
     * @returns void
     * @throws ApiError
     */
    public static oauthAuthorize(
        xTenantId: string,
        clientId: string,
        responseType: 'code',
        redirectUri: string,
        state: string,
        scope?: string,
        codeChallenge?: string,
        codeChallengeMethod?: 'S256' | 'plain',
    ): CancelablePromise<void> {
        return __request(OpenAPI, {
            method: 'GET',
            url: '/oauth/authorize',
            headers: {
                'X-Tenant-ID': xTenantId,
            },
            query: {
                'client_id': clientId,
                'response_type': responseType,
                'redirect_uri': redirectUri,
                'state': state,
                'scope': scope,
                'code_challenge': codeChallenge,
                'code_challenge_method': codeChallengeMethod,
            },
            errors: {
                302: `Redirect to login page or back with authorization code`,
            },
        });
    }
    /**
     * Refresh access token
     * Requires `X-Tenant-ID` header for tenant resolution.
     * @param xTenantId Tenant identifier. Routes the request to the correct tenant context for user lookup and authentication.
     * @param requestBody
     * @returns TokenResponse
     * @throws ApiError
     */
    public static authRefresh(
        xTenantId: string,
        requestBody: RefreshRequest,
    ): CancelablePromise<TokenResponse> {
        return __request(OpenAPI, {
            method: 'POST',
            url: '/refresh',
            headers: {
                'X-Tenant-ID': xTenantId,
            },
            body: requestBody,
            mediaType: 'application/json',
            errors: {
                401: `Invalid or expired refresh token`,
            },
        });
    }
    /**
     * User Info endpoint
     * Returns user profile claims. Requires Bearer token.
     *
     * Requires `X-Tenant-ID` header for tenant resolution.
     * @param xTenantId Tenant identifier. Routes the request to the correct tenant context for user lookup and authentication.
     * @returns UserProfile
     * @throws ApiError
     */
    public static oauthUserinfo(
        xTenantId: string,
    ): CancelablePromise<UserProfile> {
        return __request(OpenAPI, {
            method: 'GET',
            url: '/api/v1/identity/users/me/userinfo',
            headers: {
                'X-Tenant-ID': xTenantId,
            },
            errors: {
                401: `Unauthorized`,
            },
        });
    }
    /**
     * Logout all user sessions
     * Invalidates all active sessions for this user. Platform admin or user themselves.
     *
     * Requires `X-Tenant-ID` header for tenant resolution.
     * @param xTenantId Tenant identifier. Routes the request to the correct tenant context for user lookup and authentication.
     * @param userId
     * @returns void
     * @throws ApiError
     */
    public static logoutAllSessions(
        xTenantId: string,
        userId: string,
    ): CancelablePromise<void> {
        return __request(OpenAPI, {
            method: 'POST',
            url: '/users/{user_id}/logout-all-sessions',
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
     * OAuth2 logout endpoint
     * Invalidates the user's session and optionally triggers post-logout redirect.
     *
     *
     * Requires `X-Tenant-ID` header for tenant resolution.
     * @param xTenantId Tenant identifier. Routes the request to the correct tenant context for user lookup and authentication.
     * @param requestBody
     * @returns any Logout successful
     * @throws ApiError
     */
    public static oauthLogout(
        xTenantId: string,
        requestBody?: OAuthLogoutRequest,
    ): CancelablePromise<any> {
        return __request(OpenAPI, {
            method: 'POST',
            url: '/oauth/logout',
            headers: {
                'X-Tenant-ID': xTenantId,
            },
            body: requestBody,
            mediaType: 'application/json',
        });
    }
}
