/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
import type { SignupValidationResponse } from '../models/SignupValidationResponse';
import type { CancelablePromise } from '../core/CancelablePromise';
import { OpenAPI } from '../core/OpenAPI';
import { request as __request } from '../core/request';
export class SignupService {
    /**
     * Validate signup eligibility
     * Check if email/phone is allowed to register before the user starts filling forms.
     *
     * Requires `X-Tenant-ID` header for tenant resolution.
     * @param xTenantId Tenant identifier. Routes the request to the correct tenant context for user lookup and authentication.
     * @param email Email address to validate
     * @param phone Phone number in E.164 format to validate
     * @returns SignupValidationResponse Validation result
     * @throws ApiError
     */
    public static signupValidate(
        xTenantId: string,
        email?: string,
        phone?: string,
    ): CancelablePromise<SignupValidationResponse> {
        return __request(OpenAPI, {
            method: 'GET',
            url: '/signup/validate',
            headers: {
                'X-Tenant-ID': xTenantId,
            },
            query: {
                'email': email,
                'phone': phone,
            },
            errors: {
                400: `Bad request (invalid parameters)`,
            },
        });
    }
}
