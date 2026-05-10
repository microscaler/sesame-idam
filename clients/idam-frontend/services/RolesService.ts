/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
import type { PermissionListResponse } from '../models/PermissionListResponse';
import type { Role } from '../models/Role';
import type { RoleListResponse } from '../models/RoleListResponse';
import type { CancelablePromise } from '../core/CancelablePromise';
import { OpenAPI } from '../core/OpenAPI';
import { request as __request } from '../core/request';
export class RolesService {
    /**
     * List roles for application
     * Requires `X-Tenant-ID` header for tenant resolution.
     * @param xTenantId Tenant identifier. Routes the request to the correct tenant context for user lookup and authentication.
     * @param appId
     * @param page Page number for pagination
     * @param limit Number of items per page (max 100)
     * @returns RoleListResponse
     * @throws ApiError
     */
    public static listRoles(
        xTenantId: string,
        appId: string,
        page: number = 1,
        limit: number = 20,
    ): CancelablePromise<RoleListResponse> {
        return __request(OpenAPI, {
            method: 'GET',
            url: '/api/v1/am/applications/{app_id}/roles',
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
    /**
     * Get role by id
     * Requires `X-Tenant-ID` header for tenant resolution.
     * @param xTenantId Tenant identifier. Routes the request to the correct tenant context for user lookup and authentication.
     * @param appId
     * @param roleId
     * @returns Role
     * @throws ApiError
     */
    public static getRole(
        xTenantId: string,
        appId: string,
        roleId: string,
    ): CancelablePromise<Role> {
        return __request(OpenAPI, {
            method: 'GET',
            url: '/api/v1/am/applications/{app_id}/roles/{role_id}',
            path: {
                'app_id': appId,
                'role_id': roleId,
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
     * Get permissions for role
     * Requires `X-Tenant-ID` header for tenant resolution.
     * @param xTenantId Tenant identifier. Routes the request to the correct tenant context for user lookup and authentication.
     * @param appId
     * @param roleId
     * @param page Page number for pagination
     * @param limit Number of items per page (max 100)
     * @returns PermissionListResponse
     * @throws ApiError
     */
    public static getRolePermissions(
        xTenantId: string,
        appId: string,
        roleId: string,
        page: number = 1,
        limit: number = 20,
    ): CancelablePromise<PermissionListResponse> {
        return __request(OpenAPI, {
            method: 'GET',
            url: '/api/v1/am/applications/{app_id}/roles/{role_id}/permissions',
            path: {
                'app_id': appId,
                'role_id': roleId,
            },
            headers: {
                'X-Tenant-ID': xTenantId,
            },
            query: {
                'page': page,
                'limit': limit,
            },
            errors: {
                404: `Not found`,
            },
        });
    }
}
