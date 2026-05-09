/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
import type { AssignPrincipalRoleRequest } from '../models/AssignPrincipalRoleRequest';
import type { AuthorizeRequest } from '../models/AuthorizeRequest';
import type { AuthorizeResponse } from '../models/AuthorizeResponse';
import type { EffectiveRequest } from '../models/EffectiveRequest';
import type { EffectiveResponse } from '../models/EffectiveResponse';
import type { RevokePrincipalRoleRequest } from '../models/RevokePrincipalRoleRequest';
import type { SetPrincipalAttributeRequest } from '../models/SetPrincipalAttributeRequest';
import type { CancelablePromise } from '../core/CancelablePromise';
import { OpenAPI } from '../core/OpenAPI';
import { request as __request } from '../core/request';
export class PrincipalService {
    /**
     * Assign role to principal
     * Assigns a role to a principal (sub) within an application, organisation, and tenant scope.
     * SaaS customers assign roles within their own org; platform admins assign globally.
     *
     *
     * Requires `X-Tenant-ID` header for tenant resolution.
     * @param xTenantId Tenant identifier. Routes the request to the correct tenant context for user lookup and authentication.
     * @param requestBody
     * @returns void
     * @throws ApiError
     */
    public static assignPrincipalRole(
        xTenantId: string,
        requestBody: AssignPrincipalRoleRequest,
    ): CancelablePromise<void> {
        return __request(OpenAPI, {
            method: 'POST',
            url: '/principals/roles',
            headers: {
                'X-Tenant-ID': xTenantId,
            },
            body: requestBody,
            mediaType: 'application/json',
            errors: {
                400: `Invalid request`,
                404: `Application or role not found`,
            },
        });
    }
    /**
     * Revoke role from principal
     * Requires `X-Tenant-ID` header for tenant resolution.
     * @param xTenantId Tenant identifier. Routes the request to the correct tenant context for user lookup and authentication.
     * @param requestBody
     * @returns void
     * @throws ApiError
     */
    public static revokePrincipalRole(
        xTenantId: string,
        requestBody: RevokePrincipalRoleRequest,
    ): CancelablePromise<void> {
        return __request(OpenAPI, {
            method: 'DELETE',
            url: '/principals/roles',
            headers: {
                'X-Tenant-ID': xTenantId,
            },
            body: requestBody,
            mediaType: 'application/json',
            errors: {
                404: `Not found`,
            },
        });
    }
    /**
     * Set attribute for principal (ABAC)
     * Requires `X-Tenant-ID` header for tenant resolution.
     * @param xTenantId Tenant identifier. Routes the request to the correct tenant context for user lookup and authentication.
     * @param requestBody
     * @returns void
     * @throws ApiError
     */
    public static setPrincipalAttribute(
        xTenantId: string,
        requestBody: SetPrincipalAttributeRequest,
    ): CancelablePromise<void> {
        return __request(OpenAPI, {
            method: 'POST',
            url: '/principals/attributes',
            headers: {
                'X-Tenant-ID': xTenantId,
            },
            body: requestBody,
            mediaType: 'application/json',
            errors: {
                400: `Invalid request`,
            },
        });
    }
    /**
     * Get effective roles and permissions for principal
     * Returns the effective roles and permissions for a principal within an application,
     * organisation, and tenant scope. Used by Identity service for JWT enrichment.
     * SaaS customers query for their own principals; platform admins query globally.
     *
     *
     * Requires `X-Tenant-ID` header for tenant resolution.
     * @param xTenantId Tenant identifier. Routes the request to the correct tenant context for user lookup and authentication.
     * @param requestBody
     * @returns EffectiveResponse
     * @throws ApiError
     */
    public static principalEffective(
        xTenantId: string,
        requestBody: EffectiveRequest,
    ): CancelablePromise<EffectiveResponse> {
        return __request(OpenAPI, {
            method: 'POST',
            url: '/principal/effective',
            headers: {
                'X-Tenant-ID': xTenantId,
            },
            body: requestBody,
            mediaType: 'application/json',
            errors: {
                400: `Invalid request`,
                404: `Application not found`,
            },
        });
    }
    /**
     * Check if principal is allowed to perform action on resource
     * Real-time authorization check. Consuming microservices call this to verify
     * a user has permission for a specific action on a resource.
     * SaaS customers check within their own org; platform admins check globally.
     *
     *
     * Requires `X-Tenant-ID` header for tenant resolution.
     * @param xTenantId Tenant identifier. Routes the request to the correct tenant context for user lookup and authentication.
     * @param requestBody
     * @returns AuthorizeResponse
     * @throws ApiError
     */
    public static authorize(
        xTenantId: string,
        requestBody: AuthorizeRequest,
    ): CancelablePromise<AuthorizeResponse> {
        return __request(OpenAPI, {
            method: 'POST',
            url: '/authorize',
            headers: {
                'X-Tenant-ID': xTenantId,
            },
            body: requestBody,
            mediaType: 'application/json',
            errors: {
                400: `Invalid request`,
            },
        });
    }
}
