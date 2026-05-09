/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
import type { CreateUserRequest } from '../models/CreateUserRequest';
import type { EmployeeResponse } from '../models/EmployeeResponse';
import type { User } from '../models/User';
import type { UserQueryResponse } from '../models/UserQueryResponse';
import type { CancelablePromise } from '../core/CancelablePromise';
import { OpenAPI } from '../core/OpenAPI';
import { request as __request } from '../core/request';
export class UsersService {
    /**
     * Create user (idempotent by email)
     * Creates a new user. Idempotent if email matches existing user — returns existing user.
     * SaaS customers must provide their own `org_id`; platform admins may omit it.
     *
     *
     * Requires `X-Tenant-ID` header for tenant resolution.
     * @param xTenantId Tenant identifier. Routes the request to the correct tenant context for user lookup and authentication.
     * @param requestBody
     * @returns User User already exists (idempotent)
     * @throws ApiError
     */
    public static createUser(
        xTenantId: string,
        requestBody: CreateUserRequest,
    ): CancelablePromise<User> {
        return __request(OpenAPI, {
            method: 'POST',
            url: '/users',
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
     * Delete user (irreversible)
     * Irreversibly deletes a user account and all associated data.
     * Platform admin only. Users in an org can only delete themselves.
     *
     *
     * Requires `X-Tenant-ID` header for tenant resolution.
     * @param xTenantId Tenant identifier. Routes the request to the correct tenant context for user lookup and authentication.
     * @param userId
     * @returns void
     * @throws ApiError
     */
    public static deleteUser(
        xTenantId: string,
        userId: string,
    ): CancelablePromise<void> {
        return __request(OpenAPI, {
            method: 'DELETE',
            url: '/users/{user_id}',
            path: {
                'user_id': userId,
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
     * Paginated query for users with filters
     * Returns paginated list of users matching the given filters.
     * Platform admins can search across all users; SaaS customers can only search their own org.
     *
     *
     * Requires `X-Tenant-ID` header for tenant resolution.
     * @param xTenantId Tenant identifier. Routes the request to the correct tenant context for user lookup and authentication.
     * @param page
     * @param limit
     * @param emailPattern Email substring filter (case-insensitive)
     * @param emailConfirmed Filter by email confirmation status
     * @param enabled Filter by enabled status
     * @param disabled Filter by disabled status
     * @param locked Filter by locked status
     * @param createdAfter Filter by creation date (inclusive)
     * @param createdBefore Filter by creation date (inclusive)
     * @param signupFlow Filter by signup method
     * @returns UserQueryResponse Users found
     * @throws ApiError
     */
    public static queryUsers(
        xTenantId: string,
        page: number = 1,
        limit: number = 20,
        emailPattern?: string,
        emailConfirmed?: boolean | null,
        enabled?: boolean | null,
        disabled?: boolean | null,
        locked?: boolean | null,
        createdAfter?: string | null,
        createdBefore?: string | null,
        signupFlow?: 'signup' | 'invite' | 'magiclink' | 'password' | 'social' | 'saml' | null,
    ): CancelablePromise<UserQueryResponse> {
        return __request(OpenAPI, {
            method: 'GET',
            url: '/users/query',
            headers: {
                'X-Tenant-ID': xTenantId,
            },
            query: {
                'page': page,
                'limit': limit,
                'email_pattern': emailPattern,
                'email_confirmed': emailConfirmed,
                'enabled': enabled,
                'disabled': disabled,
                'locked': locked,
                'created_after': createdAfter,
                'created_before': createdBefore,
                'signup_flow': signupFlow,
            },
            errors: {
                400: `Invalid request`,
            },
        });
    }
    /**
     * Fetch user by email
     * Returns the user associated with the given email. No PII in URI.
     *
     *
     * Requires `X-Tenant-ID` header for tenant resolution.
     * @param xTenantId Tenant identifier. Routes the request to the correct tenant context for user lookup and authentication.
     * @param email
     * @returns User
     * @throws ApiError
     */
    public static fetchUserByEmail(
        xTenantId: string,
        email: string,
    ): CancelablePromise<User> {
        return __request(OpenAPI, {
            method: 'GET',
            url: '/users/email',
            headers: {
                'X-Tenant-ID': xTenantId,
            },
            query: {
                'email': email,
            },
            errors: {
                404: `Not found`,
            },
        });
    }
    /**
     * Fetch user by username
     * Requires `X-Tenant-ID` header for tenant resolution.
     * @param xTenantId Tenant identifier. Routes the request to the correct tenant context for user lookup and authentication.
     * @param username
     * @returns User
     * @throws ApiError
     */
    public static fetchUserByUsername(
        xTenantId: string,
        username: string,
    ): CancelablePromise<User> {
        return __request(OpenAPI, {
            method: 'GET',
            url: '/users/username',
            headers: {
                'X-Tenant-ID': xTenantId,
            },
            query: {
                'username': username,
            },
            errors: {
                404: `Not found`,
            },
        });
    }
    /**
     * Fetch user in employee mode
     * Returns user info filtered to the caller's organisation context only.
     * Used for B2B directory lookup — omits orgs the user is not part of.
     * Requires BearerAuth with org context.
     *
     *
     * Requires `X-Tenant-ID` header for tenant resolution.
     * @param xTenantId Tenant identifier. Routes the request to the correct tenant context for user lookup and authentication.
     * @param userId
     * @returns EmployeeResponse
     * @throws ApiError
     */
    public static fetchEmployee(
        xTenantId: string,
        userId: string,
    ): CancelablePromise<EmployeeResponse> {
        return __request(OpenAPI, {
            method: 'GET',
            url: '/users/{user_id}/employee',
            path: {
                'user_id': userId,
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
