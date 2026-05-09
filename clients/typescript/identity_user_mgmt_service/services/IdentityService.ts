/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
import type { UpdateEmailRequest } from '../models/UpdateEmailRequest';
import type { CancelablePromise } from '../core/CancelablePromise';
import { OpenAPI } from '../core/OpenAPI';
import { request as __request } from '../core/request';
export class IdentityService {
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
