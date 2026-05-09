/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
import type { TokenIssuanceRequest } from '../models/TokenIssuanceRequest';
import type { TokenResponse } from '../models/TokenResponse';
import type { CancelablePromise } from '../core/CancelablePromise';
import { OpenAPI } from '../core/OpenAPI';
import { request as __request } from '../core/request';
export class TokenIssuanceService {
    /**
     * Issue access token
     * Programmatically create tokens for server-side flows and admin scripts. Bypasses standard login flow.
     *
     * Requires `X-Tenant-ID` header for tenant resolution.
     * @param xTenantId Tenant identifier. Routes the request to the correct tenant context for user lookup and authentication.
     * @param requestBody
     * @returns TokenResponse Token issued successfully
     * @throws ApiError
     */
    public static adminIssueToken(
        xTenantId: string,
        requestBody: TokenIssuanceRequest,
    ): CancelablePromise<TokenResponse> {
        return __request(OpenAPI, {
            method: 'POST',
            url: '/api/v1/identity/users/me/token',
            headers: {
                'X-Tenant-ID': xTenantId,
            },
            body: requestBody,
            mediaType: 'application/json',
            errors: {
                403: `Forbidden (not an admin or invalid scope)`,
                404: `User not found`,
            },
        });
    }
}
