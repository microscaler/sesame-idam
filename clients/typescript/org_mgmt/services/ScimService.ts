/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
import type { ScimUser } from '../models/ScimUser';
import type { ScimUserCreateRequest } from '../models/ScimUserCreateRequest';
import type { ScimUserListResponse } from '../models/ScimUserListResponse';
import type { ScimUserUpdateRequest } from '../models/ScimUserUpdateRequest';
import type { CancelablePromise } from '../core/CancelablePromise';
import { OpenAPI } from '../core/OpenAPI';
import { request as __request } from '../core/request';
export class ScimService {
    /**
     * List SCIM users in org
     * List all provisioned users in the organization via SCIM 2.0.
     *
     * Requires `X-Tenant-ID` header for tenant resolution.
     * @param xTenantId Tenant identifier. Routes the request to the correct tenant context for user lookup and authentication.
     * @param filter SCIM filter expression (e.g., "userName eq 'user@example.com'")
     * @param count Number of results per page
     * @param page Page number for pagination
     * @param limit Number of items per page (max 100)
     * @returns ScimUserListResponse List of SCIM users
     * @throws ApiError
     */
    public static scimListUsers(
        xTenantId: string,
        filter?: string,
        count: number = 20,
        page: number = 1,
        limit: number = 20,
    ): CancelablePromise<ScimUserListResponse> {
        return __request(OpenAPI, {
            method: 'GET',
            url: '/{org_id}/scim/users',
            headers: {
                'X-Tenant-ID': xTenantId,
            },
            query: {
                'filter': filter,
                'count': count,
                'page': page,
                'limit': limit,
            },
            errors: {
                400: `Invalid filter, malformed request, or unsupported SCIM query parameter`,
                401: `Unauthorized — valid SCIM credentials required`,
                403: `Forbidden — insufficient permissions for SCIM list operations`,
                404: `Not found — org not found or SCIM provisioning disabled`,
                409: `Conflict — unsupported filter combination`,
            },
        });
    }
    /**
     * Create SCIM user in org
     * Create a new user in the organization via SCIM 2.0.
     *
     * Requires `X-Tenant-ID` header for tenant resolution.
     * @param xTenantId Tenant identifier. Routes the request to the correct tenant context for user lookup and authentication.
     * @param requestBody
     * @returns ScimUser User created
     * @throws ApiError
     */
    public static scimCreateUser(
        xTenantId: string,
        requestBody: ScimUserCreateRequest,
    ): CancelablePromise<ScimUser> {
        return __request(OpenAPI, {
            method: 'POST',
            url: '/{org_id}/scim/users',
            headers: {
                'X-Tenant-ID': xTenantId,
            },
            body: requestBody,
            mediaType: 'application/json',
            errors: {
                400: `Invalid request body — malformed SCIM user or invalid attribute values`,
                401: `Unauthorized — valid SCIM credentials required`,
                403: `Forbidden — insufficient permissions to create users`,
                404: `Not found — org not found or SCIM provisioning disabled`,
                409: `Conflict — duplicate user (email or username already exists)`,
            },
        });
    }
    /**
     * Update SCIM user in org
     * Update an existing user in the organization via SCIM 2.0.
     *
     * Requires `X-Tenant-ID` header for tenant resolution.
     * @param xTenantId Tenant identifier. Routes the request to the correct tenant context for user lookup and authentication.
     * @param userId
     * @param requestBody
     * @returns ScimUser User updated
     * @throws ApiError
     */
    public static scimUpdateUser(
        xTenantId: string,
        userId: string,
        requestBody: ScimUserUpdateRequest,
    ): CancelablePromise<ScimUser> {
        return __request(OpenAPI, {
            method: 'PUT',
            url: '/{org_id}/scim/users/{user_id}',
            path: {
                'user_id': userId,
            },
            headers: {
                'X-Tenant-ID': xTenantId,
            },
            body: requestBody,
            mediaType: 'application/json',
            errors: {
                400: `Invalid request body — malformed SCIM user or unsupported attribute mutation`,
                401: `Unauthorized — valid SCIM credentials required`,
                403: `Forbidden — insufficient permissions to update user`,
                404: `Not found — user not found or org not found`,
                409: `Conflict — version mismatch (etag) or duplicate attribute`,
            },
        });
    }
    /**
     * Delete SCIM user from org
     * Remove a user from the organization via SCIM 2.0.
     *
     * Requires `X-Tenant-ID` header for tenant resolution.
     * @param xTenantId Tenant identifier. Routes the request to the correct tenant context for user lookup and authentication.
     * @param userId
     * @returns void
     * @throws ApiError
     */
    public static scimDeleteUser(
        xTenantId: string,
        userId: string,
    ): CancelablePromise<void> {
        return __request(OpenAPI, {
            method: 'DELETE',
            url: '/{org_id}/scim/users/{user_id}',
            path: {
                'user_id': userId,
            },
            headers: {
                'X-Tenant-ID': xTenantId,
            },
            errors: {
                400: `Invalid filter — malformed delete request`,
                401: `Unauthorized — valid SCIM credentials required`,
                403: `Forbidden — insufficient permissions to delete user`,
                404: `Not found — user not found or org not found`,
                409: `Conflict — user has active dependencies preventing deletion`,
            },
        });
    }
}
