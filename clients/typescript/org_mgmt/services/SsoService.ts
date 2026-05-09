/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
import type { OidcMetadataRequest } from '../models/OidcMetadataRequest';
import type { SamlConnectionLinkResponse } from '../models/SamlConnectionLinkResponse';
import type { SamlLinkRequest } from '../models/SamlLinkRequest';
import type { ScimGroup } from '../models/ScimGroup';
import type { ScimGroupsResponse } from '../models/ScimGroupsResponse';
import type { CancelablePromise } from '../core/CancelablePromise';
import { OpenAPI } from '../core/OpenAPI';
import { request as __request } from '../core/request';
export class SsoService {
    /**
     * Allow organisation to set up SAML SSO
     * Enables SAML self-setup for this organisation. Users can configure their own SAML IdP.
     *
     *
     * Requires `X-Tenant-ID` header for tenant resolution.
     * @param xTenantId Tenant identifier. Routes the request to the correct tenant context for user lookup and authentication.
     * @param orgId
     * @returns any SAML setup allowed
     * @throws ApiError
     */
    public static allowOrgSaml(
        xTenantId: string,
        orgId: string,
    ): CancelablePromise<any> {
        return __request(OpenAPI, {
            method: 'POST',
            url: '/{org_id}/allow-saml',
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
    /**
     * Disallow organisation from using SAML SSO
     * Prevents this organisation from using SAML SSO. If SAML is already configured, it will be disabled.
     *
     *
     * Requires `X-Tenant-ID` header for tenant resolution.
     * @param xTenantId Tenant identifier. Routes the request to the correct tenant context for user lookup and authentication.
     * @param orgId
     * @returns any SAML disabled
     * @throws ApiError
     */
    public static disallowOrgSaml(
        xTenantId: string,
        orgId: string,
    ): CancelablePromise<any> {
        return __request(OpenAPI, {
            method: 'POST',
            url: '/{org_id}/disallow-saml',
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
    /**
     * Create SAML connection setup link
     * Creates a link that allows SAML setup without requiring login or account creation.
     * Useful for onboarding emails with self-serve SAML configuration.
     *
     *
     * Requires `X-Tenant-ID` header for tenant resolution.
     * @param xTenantId Tenant identifier. Routes the request to the correct tenant context for user lookup and authentication.
     * @param orgId
     * @param requestBody
     * @returns SamlConnectionLinkResponse Connection link created
     * @throws ApiError
     */
    public static createSamlLink(
        xTenantId: string,
        orgId: string,
        requestBody: SamlLinkRequest,
    ): CancelablePromise<SamlConnectionLinkResponse> {
        return __request(OpenAPI, {
            method: 'POST',
            url: '/{org_id}/create-saml-link',
            path: {
                'org_id': orgId,
            },
            headers: {
                'X-Tenant-ID': xTenantId,
            },
            body: requestBody,
            mediaType: 'application/json',
        });
    }
    /**
     * Set SAML IdP metadata for organisation
     * Imports SAML Identity Provider metadata XML to configure SSO for this organisation.
     *
     *
     * Requires `X-Tenant-ID` header for tenant resolution.
     * @param xTenantId Tenant identifier. Routes the request to the correct tenant context for user lookup and authentication.
     * @param orgId
     * @param requestBody
     * @returns any SAML IdP metadata set
     * @throws ApiError
     */
    public static setSamlIdpMetadata(
        xTenantId: string,
        orgId: string,
        requestBody: string,
    ): CancelablePromise<any> {
        return __request(OpenAPI, {
            method: 'PUT',
            url: '/{org_id}/saml-metadata',
            path: {
                'org_id': orgId,
            },
            headers: {
                'X-Tenant-ID': xTenantId,
            },
            body: requestBody,
            mediaType: 'application/xml',
            errors: {
                400: `Invalid XML`,
            },
        });
    }
    /**
     * Set OIDC IdP metadata for organisation
     * Configures OIDC Identity Provider metadata for this organisation.
     *
     *
     * Requires `X-Tenant-ID` header for tenant resolution.
     * @param xTenantId Tenant identifier. Routes the request to the correct tenant context for user lookup and authentication.
     * @param orgId
     * @param requestBody
     * @returns any OIDC metadata set
     * @throws ApiError
     */
    public static setOidcIdpMetadata(
        xTenantId: string,
        orgId: string,
        requestBody: OidcMetadataRequest,
    ): CancelablePromise<any> {
        return __request(OpenAPI, {
            method: 'POST',
            url: '/{org_id}/oidc-metadata',
            path: {
                'org_id': orgId,
            },
            headers: {
                'X-Tenant-ID': xTenantId,
            },
            body: requestBody,
            mediaType: 'application/json',
            errors: {
                400: `Invalid metadata`,
            },
        });
    }
    /**
     * Enable SAML connection for organisation
     * Activates the SAML connection so users can authenticate via SSO.
     *
     *
     * Requires `X-Tenant-ID` header for tenant resolution.
     * @param xTenantId Tenant identifier. Routes the request to the correct tenant context for user lookup and authentication.
     * @param orgId
     * @returns any SAML enabled
     * @throws ApiError
     */
    public static enableSaml(
        xTenantId: string,
        orgId: string,
    ): CancelablePromise<any> {
        return __request(OpenAPI, {
            method: 'POST',
            url: '/{org_id}/enable-saml',
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
    /**
     * Delete SAML connection
     * Removes the SAML configuration from this organisation.
     *
     * Requires `X-Tenant-ID` header for tenant resolution.
     * @param xTenantId Tenant identifier. Routes the request to the correct tenant context for user lookup and authentication.
     * @param orgId
     * @returns void
     * @throws ApiError
     */
    public static deleteSaml(
        xTenantId: string,
        orgId: string,
    ): CancelablePromise<void> {
        return __request(OpenAPI, {
            method: 'DELETE',
            url: '/{org_id}/saml',
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
    /**
     * Migrate organisation to isolated SAML mode
     * Moves the organisation to an isolated identity pool, separating SAML users from the main user database.
     * Platform admin only.
     *
     *
     * Requires `X-Tenant-ID` header for tenant resolution.
     * @param xTenantId Tenant identifier. Routes the request to the correct tenant context for user lookup and authentication.
     * @param orgId
     * @returns any Organisation migrated to isolated mode
     * @throws ApiError
     */
    public static migrateOrgIsolated(
        xTenantId: string,
        orgId: string,
    ): CancelablePromise<any> {
        return __request(OpenAPI, {
            method: 'POST',
            url: '/{org_id}/migrate-to-isolated',
            path: {
                'org_id': orgId,
            },
            headers: {
                'X-Tenant-ID': xTenantId,
            },
            errors: {
                400: `Invalid request`,
                404: `Not found`,
            },
        });
    }
    /**
     * Fetch SCIM groups for organisation
     * Returns paginated list of SCIM groups for this organisation.
     * Used for SCIM provisioning integrations.
     *
     *
     * Requires `X-Tenant-ID` header for tenant resolution.
     * @param xTenantId Tenant identifier. Routes the request to the correct tenant context for user lookup and authentication.
     * @param orgId
     * @param pageSize
     * @param pageNumber
     * @returns ScimGroupsResponse SCIM groups
     * @throws ApiError
     */
    public static fetchScimGroups(
        xTenantId: string,
        orgId: string,
        pageSize: number = 10,
        pageNumber?: number,
    ): CancelablePromise<ScimGroupsResponse> {
        return __request(OpenAPI, {
            method: 'GET',
            url: '/{org_id}/scim/groups',
            path: {
                'org_id': orgId,
            },
            headers: {
                'X-Tenant-ID': xTenantId,
            },
            query: {
                'page_size': pageSize,
                'page_number': pageNumber,
            },
            errors: {
                400: `Invalid SCIM filter`,
            },
        });
    }
    /**
     * Fetch a specific SCIM group
     * Returns a specific SCIM group by ID with member details.
     *
     * Requires `X-Tenant-ID` header for tenant resolution.
     * @param xTenantId Tenant identifier. Routes the request to the correct tenant context for user lookup and authentication.
     * @param orgId
     * @param groupId
     * @param page Page number for pagination
     * @param limit Number of items per page (max 100)
     * @returns ScimGroup SCIM group
     * @throws ApiError
     */
    public static fetchScimGroup(
        xTenantId: string,
        orgId: string,
        groupId: string,
        page: number = 1,
        limit: number = 20,
    ): CancelablePromise<ScimGroup> {
        return __request(OpenAPI, {
            method: 'GET',
            url: '/{org_id}/scim/groups/{group_id}',
            path: {
                'org_id': orgId,
                'group_id': groupId,
            },
            headers: {
                'X-Tenant-ID': xTenantId,
            },
            query: {
                'page': page,
                'limit': limit,
            },
            errors: {
                400: `Invalid SCIM filter`,
                404: `Not found`,
            },
        });
    }
}
