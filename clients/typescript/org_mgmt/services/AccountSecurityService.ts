/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
import type { InvalidateKeysResponse } from '../models/InvalidateKeysResponse';
import type { CancelablePromise } from '../core/CancelablePromise';
import { OpenAPI } from '../core/OpenAPI';
import { request as __request } from '../core/request';
export class AccountSecurityService {
    /**
     * Invalidate all API keys for user
     * Called when user is blocked or deleted. Archives all API keys belonging to this user across personal and org scopes.
     *
     * Requires `X-Tenant-ID` header for tenant resolution.
     * @param xTenantId Tenant identifier. Routes the request to the correct tenant context for user lookup and authentication.
     * @param userId
     * @returns InvalidateKeysResponse API keys invalidated
     * @throws ApiError
     */
    public static invalidateUserApiKeys(
        xTenantId: string,
        userId: string,
    ): CancelablePromise<InvalidateKeysResponse> {
        return __request(OpenAPI, {
            method: 'POST',
            url: '/admin/users/{user_id}/invalidate-all-keys',
            path: {
                'user_id': userId,
            },
            headers: {
                'X-Tenant-ID': xTenantId,
            },
            errors: {
                403: `Forbidden (not an admin)`,
            },
        });
    }
}
