/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
import type { UpdateEmailRequest } from '../models/UpdateEmailRequest';
import type { UpdateUserProfileRequest } from '../models/UpdateUserProfileRequest';
import type { UserProfile } from '../models/UserProfile';
import type { CancelablePromise } from '../core/CancelablePromise';
import { OpenAPI } from '../core/OpenAPI';
import { request as __request } from '../core/request';
export class IdentityService {
    /**
     * Current user profile
     * Requires `X-Tenant-ID` header for tenant resolution.
     * @param xTenantId Tenant identifier. Routes the request to the correct tenant context for user lookup and authentication.
     * @returns UserProfile
     * @throws ApiError
     */
    public static usersMeGet(
        xTenantId: string,
    ): CancelablePromise<UserProfile> {
        return __request(OpenAPI, {
            method: 'GET',
            url: '/api/v1/identity/users/me',
            headers: {
                'X-Tenant-ID': xTenantId,
            },
            errors: {
                401: `Unauthorized`,
            },
        });
    }
    /**
     * Update current user profile
     * Requires `X-Tenant-ID` header for tenant resolution.
     * @param xTenantId Tenant identifier. Routes the request to the correct tenant context for user lookup and authentication.
     * @param requestBody
     * @returns UserProfile
     * @throws ApiError
     */
    public static usersMePatch(
        xTenantId: string,
        requestBody?: UpdateUserProfileRequest,
    ): CancelablePromise<UserProfile> {
        return __request(OpenAPI, {
            method: 'PATCH',
            url: '/api/v1/identity/users/me',
            headers: {
                'X-Tenant-ID': xTenantId,
            },
            body: requestBody,
            mediaType: 'application/json',
            errors: {
                401: `Unauthorized`,
            },
        });
    }
    /**
     * Change user email
     * Requires `X-Tenant-ID` header for tenant resolution.
     * @param xTenantId Tenant identifier. Routes the request to the correct tenant context for user lookup and authentication.
     * @param userId
     * @param requestBody
     * @returns any Email updated
     * @throws ApiError
     */
    public static updateUserEmail(
        xTenantId: string,
        userId: string,
        requestBody: UpdateEmailRequest,
    ): CancelablePromise<any> {
        return __request(OpenAPI, {
            method: 'PUT',
            url: '/users/{user_id}/email',
            path: {
                'user_id': userId,
            },
            headers: {
                'X-Tenant-ID': xTenantId,
            },
            body: requestBody,
            mediaType: 'application/json',
            errors: {
                400: `Invalid request`,
                404: `Not found`,
            },
        });
    }
}
