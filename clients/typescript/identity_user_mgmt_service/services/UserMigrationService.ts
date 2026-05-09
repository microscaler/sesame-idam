/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
import type { MigratePasswordRequest } from '../models/MigratePasswordRequest';
import type { MigrateUserRequest } from '../models/MigrateUserRequest';
import type { User } from '../models/User';
import type { CancelablePromise } from '../core/CancelablePromise';
import { OpenAPI } from '../core/OpenAPI';
import { request as __request } from '../core/request';
export class UserMigrationService {
    /**
     * Migrate user from external auth system
     * Imports a user from another authentication system. Includes hash+salt password import,
     * extra properties, legacy user ID mapping. Platform admin only.
     *
     *
     * Requires `X-Tenant-ID` header for tenant resolution.
     * @param xTenantId Tenant identifier. Routes the request to the correct tenant context for user lookup and authentication.
     * @param requestBody
     * @returns User User migrated
     * @throws ApiError
     */
    public static migrateUser(
        xTenantId: string,
        requestBody: MigrateUserRequest,
    ): CancelablePromise<User> {
        return __request(OpenAPI, {
            method: 'POST',
            url: '/users/migrate',
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
     * Bulk migrate passwords (hash+salt)
     * Imports passwords from another system. Password hash format: $2a$14$... (bcrypt).
     * Platform admin only.
     *
     *
     * Requires `X-Tenant-ID` header for tenant resolution.
     * @param xTenantId Tenant identifier. Routes the request to the correct tenant context for user lookup and authentication.
     * @param requestBody
     * @returns any Passwords migrated
     * @throws ApiError
     */
    public static migrateUserPasswords(
        xTenantId: string,
        requestBody: MigratePasswordRequest,
    ): CancelablePromise<any> {
        return __request(OpenAPI, {
            method: 'POST',
            url: '/users/migrate-password',
            headers: {
                'X-Tenant-ID': xTenantId,
            },
            body: requestBody,
            mediaType: 'application/json',
            errors: {
                400: `Invalid hash format`,
            },
        });
    }
}
