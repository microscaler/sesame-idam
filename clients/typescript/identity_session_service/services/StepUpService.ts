/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
import type { StepUpRequest } from '../models/StepUpRequest';
import type { StepUpResponse } from '../models/StepUpResponse';
import type { CancelablePromise } from '../core/CancelablePromise';
import { OpenAPI } from '../core/OpenAPI';
import { request as __request } from '../core/request';
export class StepUpService {
    /**
     * Step-up MFA verification
     * Re-authenticate for sensitive operations (delete account, change email, etc.). Requires current MFA enrollment.
     *
     * Requires `X-Tenant-ID` header for tenant resolution.
     * @param xTenantId Tenant identifier. Routes the request to the correct tenant context for user lookup and authentication.
     * @param requestBody
     * @returns StepUpResponse Step-up verification successful, session elevated
     * @throws ApiError
     */
    public static stepUpVerify(
        xTenantId: string,
        requestBody: StepUpRequest,
    ): CancelablePromise<StepUpResponse> {
        return __request(OpenAPI, {
            method: 'POST',
            url: '/verify/step-up',
            headers: {
                'X-Tenant-ID': xTenantId,
            },
            body: requestBody,
            mediaType: 'application/json',
            errors: {
                401: `Unauthorized or MFA verification failed`,
            },
        });
    }
}
