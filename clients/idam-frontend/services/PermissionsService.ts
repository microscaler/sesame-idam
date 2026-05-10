/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
import type { PermissionListResponse } from '../models/PermissionListResponse';
import type { CancelablePromise } from '../core/CancelablePromise';
import { OpenAPI } from '../core/OpenAPI';
import { request as __request } from '../core/request';
export class PermissionsService {
    /**
     * List permissions for application
     * Requires `X-Tenant-ID` header for tenant resolution.
     * @param xTenantId Tenant identifier. Routes the request to the correct tenant context for user lookup and authentication.
     * @param appId
     * @param page Page number for pagination
     * @param limit Number of items per page (max 100)
     * @returns PermissionListResponse
     * @throws ApiError
     */
    public static listPermissions(
        xTenantId: string,
        appId: string,
        page: number = 1,
        limit: number = 20,
    ): CancelablePromise<PermissionListResponse> {
        return __request(OpenAPI, {
            method: 'GET',
            url: '/api/v1/am/applications/{app_id}/permissions',
            path: {
                'app_id': appId,
            },
            headers: {
                'X-Tenant-ID': xTenantId,
            },
            query: {
                'page': page,
                'limit': limit,
            },
        });
    }
}
