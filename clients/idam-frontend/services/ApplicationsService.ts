/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
import type { Application } from '../models/Application';
import type { ApplicationListResponse } from '../models/ApplicationListResponse';
import type { CancelablePromise } from '../core/CancelablePromise';
import { OpenAPI } from '../core/OpenAPI';
import { request as __request } from '../core/request';
export class ApplicationsService {
    /**
     * List applications
     * Requires `X-Tenant-ID` header for tenant resolution.
     * @param xTenantId Tenant identifier. Routes the request to the correct tenant context for user lookup and authentication.
     * @param page Page number for pagination
     * @param limit Number of items per page (max 100)
     * @returns ApplicationListResponse
     * @throws ApiError
     */
    public static listApplications(
        xTenantId: string,
        page: number = 1,
        limit: number = 20,
    ): CancelablePromise<ApplicationListResponse> {
        return __request(OpenAPI, {
            method: 'GET',
            url: '/api/v1/am/applications',
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
     * Get application by id
     * Requires `X-Tenant-ID` header for tenant resolution.
     * @param xTenantId Tenant identifier. Routes the request to the correct tenant context for user lookup and authentication.
     * @param appId
     * @returns Application
     * @throws ApiError
     */
    public static getApplication(
        xTenantId: string,
        appId: string,
    ): CancelablePromise<Application> {
        return __request(OpenAPI, {
            method: 'GET',
            url: '/api/v1/am/applications/{app_id}',
            path: {
                'app_id': appId,
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
