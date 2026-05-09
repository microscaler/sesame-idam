/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
import type { OAuthLogoutRequest } from '../models/OAuthLogoutRequest';
import type { CancelablePromise } from '../core/CancelablePromise';
import { OpenAPI } from '../core/OpenAPI';
import { request as __request } from '../core/request';
export class SessionsService {
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
