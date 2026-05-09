/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
import type { ImpersonateRequest } from '../models/ImpersonateRequest';
import type { ImpersonateResponse } from '../models/ImpersonateResponse';
import type { ImpersonateRestoreRequest } from '../models/ImpersonateRestoreRequest';
import type { CancelablePromise } from '../core/CancelablePromise';
import { OpenAPI } from '../core/OpenAPI';
import { request as __request } from '../core/request';
export class ImpersonationService {
    /**
     * Impersonate user
     * Admin switches to user session for debugging/support. Creates a new session for the target user.
     *
     * Requires `X-Tenant-ID` header for tenant resolution.
     * @param xTenantId Tenant identifier. Routes the request to the correct tenant context for user lookup and authentication.
     * @param requestBody
     * @returns ImpersonateResponse Impersonation successful, new session created
     * @throws ApiError
     */
    public static adminImpersonate(
        xTenantId: string,
        requestBody: ImpersonateRequest,
    ): CancelablePromise<ImpersonateResponse> {
        return __request(OpenAPI, {
            method: 'POST',
            url: '/admin/impersonate',
            headers: {
                'X-Tenant-ID': xTenantId,
            },
            body: requestBody,
            mediaType: 'application/json',
            errors: {
                403: `Forbidden (not an admin)`,
                404: `User not found`,
            },
        });
    }
    /**
     * Restore admin session
     * Switch back from impersonated user session to admin session.
     *
     * Requires `X-Tenant-ID` header for tenant resolution.
     * @param xTenantId Tenant identifier. Routes the request to the correct tenant context for user lookup and authentication.
     * @param requestBody
     * @returns ImpersonateResponse Restored to admin session
     * @throws ApiError
     */
    public static adminRestoreImpersonation(
        xTenantId: string,
        requestBody: ImpersonateRestoreRequest,
    ): CancelablePromise<ImpersonateResponse> {
        return __request(OpenAPI, {
            method: 'POST',
            url: '/admin/impersonate/restore',
            headers: {
                'X-Tenant-ID': xTenantId,
            },
            body: requestBody,
            mediaType: 'application/json',
            errors: {
                403: `Forbidden (not impersonating or not admin)`,
            },
        });
    }
}
