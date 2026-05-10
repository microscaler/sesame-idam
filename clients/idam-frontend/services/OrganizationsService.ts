/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
import type { Org } from '../models/Org';
import type { CancelablePromise } from '../core/CancelablePromise';
import { OpenAPI } from '../core/OpenAPI';
import { request as __request } from '../core/request';
export class OrganizationsService {
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
}
