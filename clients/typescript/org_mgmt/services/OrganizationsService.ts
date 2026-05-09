/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
import type { Org } from '../models/Org';
import type { OrgDomainsRequest } from '../models/OrgDomainsRequest';
import type { OrgListResponse } from '../models/OrgListResponse';
import type { UpdateOrgRequest } from '../models/UpdateOrgRequest';
import type { CancelablePromise } from '../core/CancelablePromise';
import { OpenAPI } from '../core/OpenAPI';
import { request as __request } from '../core/request';
export class OrganizationsService {
    /**
     * Query for organisations
     * Paginated search for organisations. SaaS customers see only their own orgs.
     * Platform admins see all orgs.
     *
     *
     * Requires `X-Tenant-ID` header for tenant resolution.
     * @param xTenantId Tenant identifier. Routes the request to the correct tenant context for user lookup and authentication.
     * @param pageSize
     * @param pageNumber
     * @param orderBy
     * @param name Name filter (substring match)
     * @param domain Filter by domain match
     * @param legacyOrgId Filter by legacy ID from previous auth system
     * @returns OrgListResponse Organisation list
     * @throws ApiError
     */
    public static queryOrgs(
        xTenantId: string,
        pageSize: number = 10,
        pageNumber?: number,
        orderBy?: 'CREATED_AT_ASC' | 'CREATED_AT_DESC' | 'NAME',
        name?: string,
        domain?: string,
        legacyOrgId?: string,
    ): CancelablePromise<OrgListResponse> {
        return __request(OpenAPI, {
            method: 'GET',
            url: '/',
            headers: {
                'X-Tenant-ID': xTenantId,
            },
            query: {
                'page_size': pageSize,
                'page_number': pageNumber,
                'order_by': orderBy,
                'name': name,
                'domain': domain,
                'legacy_org_id': legacyOrgId,
            },
        });
    }
    /**
     * Fetch organisation by ID
     * Returns organisation details including SAML config, domain, metadata.
     *
     * Requires `X-Tenant-ID` header for tenant resolution.
     * @param xTenantId Tenant identifier. Routes the request to the correct tenant context for user lookup and authentication.
     * @param orgId
     * @returns Org Organisation found
     * @throws ApiError
     */
    public static fetchOrg(
        xTenantId: string,
        orgId: string,
    ): CancelablePromise<Org> {
        return __request(OpenAPI, {
            method: 'GET',
            url: '/{org_id}',
            path: {
                'org_id': orgId,
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
     * Update organisation
     * Updates organisation fields. SaaS customers can only update their own orgs.
     *
     *
     * Requires `X-Tenant-ID` header for tenant resolution.
     * @param xTenantId Tenant identifier. Routes the request to the correct tenant context for user lookup and authentication.
     * @param orgId
     * @param requestBody
     * @returns any Organisation updated
     * @throws ApiError
     */
    public static updateOrg(
        xTenantId: string,
        orgId: string,
        requestBody: UpdateOrgRequest,
    ): CancelablePromise<any> {
        return __request(OpenAPI, {
            method: 'PUT',
            url: '/{org_id}',
            path: {
                'org_id': orgId,
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
    /**
     * Delete organisation
     * Irreversible deletion of organisation.
     *
     * Requires `X-Tenant-ID` header for tenant resolution.
     * @param xTenantId Tenant identifier. Routes the request to the correct tenant context for user lookup and authentication.
     * @param orgId
     * @returns void
     * @throws ApiError
     */
    public static deleteOrg(
        xTenantId: string,
        orgId: string,
    ): CancelablePromise<void> {
        return __request(OpenAPI, {
            method: 'DELETE',
            url: '/{org_id}',
            path: {
                'org_id': orgId,
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
     * Update organisation domain settings
     * Configures domain-based auto-join, restrictions, and extra domains for the organisation.
     *
     *
     * Requires `X-Tenant-ID` header for tenant resolution.
     * @param xTenantId Tenant identifier. Routes the request to the correct tenant context for user lookup and authentication.
     * @param orgId
     * @param requestBody
     * @returns any Domains updated
     * @throws ApiError
     */
    public static updateOrgDomains(
        xTenantId: string,
        orgId: string,
        requestBody: OrgDomainsRequest,
    ): CancelablePromise<any> {
        return __request(OpenAPI, {
            method: 'PUT',
            url: '/{org_id}/domains',
            path: {
                'org_id': orgId,
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
