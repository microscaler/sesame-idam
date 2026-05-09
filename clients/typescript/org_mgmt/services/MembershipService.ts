/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
import type { AddUserToOrgRequest } from '../models/AddUserToOrgRequest';
import type { ChangeUserRoleRequest } from '../models/ChangeUserRoleRequest';
import type { InviteUserToOrgByIdRequest } from '../models/InviteUserToOrgByIdRequest';
import type { InviteUserToOrgRequest } from '../models/InviteUserToOrgRequest';
import type { RemoveUserFromOrgRequest } from '../models/RemoveUserFromOrgRequest';
import type { RevokeInviteRequest } from '../models/RevokeInviteRequest';
import type { RoleMappingResponse } from '../models/RoleMappingResponse';
import type { SubscribeRoleMappingRequest } from '../models/SubscribeRoleMappingRequest';
import type { UsersInOrgResponse } from '../models/UsersInOrgResponse';
import type { CancelablePromise } from '../core/CancelablePromise';
import { OpenAPI } from '../core/OpenAPI';
import { request as __request } from '../core/request';
export class MembershipService {
    /**
     * Fetch users in organisation
     * Returns users in this organisation with optional role filter.
     * SaaS customers see only users in their own orgs.
     *
     *
     * Requires `X-Tenant-ID` header for tenant resolution.
     * @param xTenantId Tenant identifier. Routes the request to the correct tenant context for user lookup and authentication.
     * @param orgId
     * @param role Filter by role (case-sensitive)
     * @param includeOrgs Include all orgs each user belongs to in response
     * @param pageSize
     * @param pageNumber
     * @returns UsersInOrgResponse Users in organisation
     * @throws ApiError
     */
    public static fetchUsersInOrg(
        xTenantId: string,
        orgId: string,
        role?: string,
        includeOrgs?: boolean,
        pageSize: number = 10,
        pageNumber?: number,
    ): CancelablePromise<UsersInOrgResponse> {
        return __request(OpenAPI, {
            method: 'GET',
            url: '/{org_id}/users',
            path: {
                'org_id': orgId,
            },
            headers: {
                'X-Tenant-ID': xTenantId,
            },
            query: {
                'role': role,
                'include_orgs': includeOrgs,
                'page_size': pageSize,
                'page_number': pageNumber,
            },
        });
    }
    /**
     * Add user to organisation
     * Adds an existing user to this organisation with a specified role.
     * SaaS customers can only add users to their own orgs.
     *
     *
     * Requires `X-Tenant-ID` header for tenant resolution.
     * @param xTenantId Tenant identifier. Routes the request to the correct tenant context for user lookup and authentication.
     * @param orgId
     * @param userId
     * @param requestBody
     * @returns any User added
     * @throws ApiError
     */
    public static addUserToOrg(
        xTenantId: string,
        orgId: string,
        userId: string,
        requestBody: AddUserToOrgRequest,
    ): CancelablePromise<any> {
        return __request(OpenAPI, {
            method: 'POST',
            url: '/{org_id}/users',
            path: {
                'org_id': orgId,
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
    /**
     * Invite user to organisation by email
     * Sends an email invitation to join this organisation with a specified role.
     * The invitee does not need to exist as a user yet.
     *
     *
     * Requires `X-Tenant-ID` header for tenant resolution.
     * @param xTenantId Tenant identifier. Routes the request to the correct tenant context for user lookup and authentication.
     * @param orgId
     * @param requestBody
     * @returns any Invite sent
     * @throws ApiError
     */
    public static inviteUserToOrg(
        xTenantId: string,
        orgId: string,
        requestBody: InviteUserToOrgRequest,
    ): CancelablePromise<any> {
        return __request(OpenAPI, {
            method: 'POST',
            url: '/{org_id}/invite-user',
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
            },
        });
    }
    /**
     * Invite existing user to organisation
     * Invites an already-registered user to join this organisation.
     * Sends an email notification.
     *
     *
     * Requires `X-Tenant-ID` header for tenant resolution.
     * @param xTenantId Tenant identifier. Routes the request to the correct tenant context for user lookup and authentication.
     * @param orgId
     * @param requestBody
     * @returns any Invite sent
     * @throws ApiError
     */
    public static inviteUserToOrgById(
        xTenantId: string,
        orgId: string,
        requestBody: InviteUserToOrgByIdRequest,
    ): CancelablePromise<any> {
        return __request(OpenAPI, {
            method: 'POST',
            url: '/{org_id}/invite-user-by-id',
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
            },
        });
    }
    /**
     * Revoke pending organisation invite
     * Cancels a pending invite. The invitee will no longer be able to accept.
     *
     *
     * Requires `X-Tenant-ID` header for tenant resolution.
     * @param xTenantId Tenant identifier. Routes the request to the correct tenant context for user lookup and authentication.
     * @param orgId
     * @param requestBody
     * @returns void
     * @throws ApiError
     */
    public static revokePendingInvite(
        xTenantId: string,
        orgId: string,
        requestBody: RevokeInviteRequest,
    ): CancelablePromise<void> {
        return __request(OpenAPI, {
            method: 'DELETE',
            url: '/{org_id}/pending-invites',
            path: {
                'org_id': orgId,
            },
            headers: {
                'X-Tenant-ID': xTenantId,
            },
            body: requestBody,
            mediaType: 'application/json',
            errors: {
                404: `Not found`,
            },
        });
    }
    /**
     * Fetch custom role mappings for organisation
     * Returns the role configuration (e.g. "Free Plan", "Paid Plan") subscribed to this organisation.
     * Plan-based roles automatically assign roles/permissions to all org members.
     *
     *
     * Requires `X-Tenant-ID` header for tenant resolution.
     * @param xTenantId Tenant identifier. Routes the request to the correct tenant context for user lookup and authentication.
     * @param orgId
     * @param page Page number for pagination
     * @param limit Number of items per page (max 100)
     * @returns RoleMappingResponse Role mappings
     * @throws ApiError
     */
    public static fetchRoleMappings(
        xTenantId: string,
        orgId: string,
        page: number = 1,
        limit: number = 20,
    ): CancelablePromise<RoleMappingResponse> {
        return __request(OpenAPI, {
            method: 'GET',
            url: '/{org_id}/role-mappings',
            path: {
                'org_id': orgId,
            },
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
     * Subscribe organisation to a role mapping
     * Links an organisation to a plan-based role configuration.
     * E.g., "Paid Plan" → all members get Admin + Member roles.
     *
     *
     * Requires `X-Tenant-ID` header for tenant resolution.
     * @param xTenantId Tenant identifier. Routes the request to the correct tenant context for user lookup and authentication.
     * @param orgId
     * @param requestBody
     * @returns any Subscription updated
     * @throws ApiError
     */
    public static subscribeOrgToRoleMapping(
        xTenantId: string,
        orgId: string,
        requestBody: SubscribeRoleMappingRequest,
    ): CancelablePromise<any> {
        return __request(OpenAPI, {
            method: 'PUT',
            url: '/{org_id}/subscribe-role-mapping',
            path: {
                'org_id': orgId,
            },
            headers: {
                'X-Tenant-ID': xTenantId,
            },
            body: requestBody,
            mediaType: 'application/json',
            errors: {
                400: `Invalid mapping name`,
                404: `Not found`,
            },
        });
    }
    /**
     * Remove user from organisation
     * Removes a user from this organisation. The user retains their account and may belong to other orgs.
     *
     *
     * Requires `X-Tenant-ID` header for tenant resolution.
     * @param xTenantId Tenant identifier. Routes the request to the correct tenant context for user lookup and authentication.
     * @param orgId
     * @param userId
     * @param requestBody
     * @returns void
     * @throws ApiError
     */
    public static removeUserFromOrg(
        xTenantId: string,
        orgId: string,
        userId: string,
        requestBody: RemoveUserFromOrgRequest,
    ): CancelablePromise<void> {
        return __request(OpenAPI, {
            method: 'DELETE',
            url: '/{org_id}/users/{user_id}',
            path: {
                'org_id': orgId,
                'user_id': userId,
            },
            headers: {
                'X-Tenant-ID': xTenantId,
            },
            body: requestBody,
            mediaType: 'application/json',
            errors: {
                404: `Not found`,
            },
        });
    }
    /**
     * Change user role in organisation
     * Changes a user's primary role and/or additional roles within this organisation.
     * Multi-role support allows a user to hold multiple roles in a single org.
     *
     *
     * Requires `X-Tenant-ID` header for tenant resolution.
     * @param xTenantId Tenant identifier. Routes the request to the correct tenant context for user lookup and authentication.
     * @param orgId
     * @param userId
     * @param requestBody
     * @returns any Role changed
     * @throws ApiError
     */
    public static changeUserRoleInOrg(
        xTenantId: string,
        orgId: string,
        userId: string,
        requestBody: ChangeUserRoleRequest,
    ): CancelablePromise<any> {
        return __request(OpenAPI, {
            method: 'PATCH',
            url: '/{org_id}/users/{user_id}/role',
            path: {
                'org_id': orgId,
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
