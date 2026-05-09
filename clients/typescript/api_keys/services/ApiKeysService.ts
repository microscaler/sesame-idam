/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
import type { ApiKey } from '../models/ApiKey';
import type { ApiKeyCreateResponse } from '../models/ApiKeyCreateResponse';
import type { ApiKeyListResponse } from '../models/ApiKeyListResponse';
import type { ApiKeyUsageResponse } from '../models/ApiKeyUsageResponse';
import type { ApiKeyValidationResponse } from '../models/ApiKeyValidationResponse';
import type { ArchivedApiKey } from '../models/ArchivedApiKey';
import type { ArchivedApiKeyListResponse } from '../models/ArchivedApiKeyListResponse';
import type { CreateApiKeyRequest } from '../models/CreateApiKeyRequest';
import type { ImportApiKeysRequest } from '../models/ImportApiKeysRequest';
import type { ImportApiKeysResponse } from '../models/ImportApiKeysResponse';
import type { OrgApiKeyValidationResponse } from '../models/OrgApiKeyValidationResponse';
import type { PersonalApiKeyValidationResponse } from '../models/PersonalApiKeyValidationResponse';
import type { UpdateApiKeyRequest } from '../models/UpdateApiKeyRequest';
import type { ValidateApiKeyRequest } from '../models/ValidateApiKeyRequest';
import type { CancelablePromise } from '../core/CancelablePromise';
import { OpenAPI } from '../core/OpenAPI';
import { request as __request } from '../core/request';
export class ApiKeysService {
    /**
     * Create API key (M2M key / service account)
     * Creates a machine-to-machine API key for a user or organisation.
     * Separate from the Sesame platform API key.
     * SaaS customers can only create keys for their own users/orgs.
     *
     *
     * Requires `X-Tenant-ID` header for tenant resolution.
     * @param xTenantId Tenant identifier. Routes the request to the correct tenant context for user lookup and authentication.
     * @param requestBody
     * @returns ApiKeyCreateResponse API key created
     * @throws ApiError
     */
    public static createApiKey(
        xTenantId: string,
        requestBody: CreateApiKeyRequest,
    ): CancelablePromise<ApiKeyCreateResponse> {
        return __request(OpenAPI, {
            method: 'POST',
            url: '/',
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
     * Fetch active API keys
     * Returns paginated list of active (non-expired) API keys.
     * SaaS customers can filter by their own users/orgs.
     *
     *
     * Requires `X-Tenant-ID` header for tenant resolution.
     * @param xTenantId Tenant identifier. Routes the request to the correct tenant context for user lookup and authentication.
     * @param userId Filter by API key owner user ID
     * @param userEmail Filter by API key owner email
     * @param orgId Filter by API key owner organisation ID
     * @param pageSize
     * @param pageNumber
     * @returns ApiKeyListResponse Active API keys
     * @throws ApiError
     */
    public static fetchActiveApiKeys(
        xTenantId: string,
        userId?: string,
        userEmail?: string,
        orgId?: string,
        pageSize: number = 10,
        pageNumber?: number,
    ): CancelablePromise<ApiKeyListResponse> {
        return __request(OpenAPI, {
            method: 'GET',
            url: '/current',
            headers: {
                'X-Tenant-ID': xTenantId,
            },
            query: {
                'user_id': userId,
                'user_email': userEmail,
                'org_id': orgId,
                'page_size': pageSize,
                'page_number': pageNumber,
            },
        });
    }
    /**
     * Update API key metadata
     * Update the name, metadata, or expiration of an existing API key.
     *
     * Requires `X-Tenant-ID` header for tenant resolution.
     * @param xTenantId Tenant identifier. Routes the request to the correct tenant context for user lookup and authentication.
     * @param keyId
     * @param requestBody
     * @returns ApiKey API key updated
     * @throws ApiError
     */
    public static updateApiKey(
        xTenantId: string,
        keyId: string,
        requestBody: UpdateApiKeyRequest,
    ): CancelablePromise<ApiKey> {
        return __request(OpenAPI, {
            method: 'PUT',
            url: '/{key_id}',
            path: {
                'key_id': keyId,
            },
            headers: {
                'X-Tenant-ID': xTenantId,
            },
            body: requestBody,
            mediaType: 'application/json',
            errors: {
                400: `Invalid request`,
                404: `API key not found`,
            },
        });
    }
    /**
     * Delete API key
     * Revokes an API key. All uses after deletion will be rejected.
     *
     * Requires `X-Tenant-ID` header for tenant resolution.
     * @param xTenantId Tenant identifier. Routes the request to the correct tenant context for user lookup and authentication.
     * @param keyId
     * @returns void
     * @throws ApiError
     */
    public static deleteApiKey(
        xTenantId: string,
        keyId: string,
    ): CancelablePromise<void> {
        return __request(OpenAPI, {
            method: 'DELETE',
            url: '/{key_id}',
            path: {
                'key_id': keyId,
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
     * Validate API key
     * Validates an API key and returns the associated user, organisation, and permissions.
     * Used by consuming services to authenticate incoming API key requests.
     *
     * Optional `key_type` query parameter: "any" (default), "personal", or "org".
     * If specified and the key doesn't match the requested type, returns 401.
     *
     * Endpoints `/validate/personal` and `/validate/org` are deprecated — use `/validate?key_type=personal` or `/validate?key_type=org` instead.
     *
     *
     * Requires `X-Tenant-ID` header for tenant resolution.
     * @param xTenantId Tenant identifier. Routes the request to the correct tenant context for user lookup and authentication.
     * @param requestBody
     * @param keyType Filter validation to a specific key type. "any" validates all key types.
     * @returns ApiKeyValidationResponse API key is valid
     * @throws ApiError
     */
    public static validateApiKey(
        xTenantId: string,
        requestBody: ValidateApiKeyRequest,
        keyType: 'any' | 'personal' | 'org' = 'any',
    ): CancelablePromise<ApiKeyValidationResponse> {
        return __request(OpenAPI, {
            method: 'POST',
            url: '/validate',
            headers: {
                'X-Tenant-ID': xTenantId,
            },
            query: {
                'key_type': keyType,
            },
            body: requestBody,
            mediaType: 'application/json',
            errors: {
                401: `Invalid or expired API key`,
            },
        });
    }
    /**
     * Fetch API key usage
     * Returns the number of API key validation calls for a given key/user/org on a specific date.
     *
     *
     * Requires `X-Tenant-ID` header for tenant resolution.
     * @param xTenantId Tenant identifier. Routes the request to the correct tenant context for user lookup and authentication.
     * @param date Date filter in YYYY-MM-DD format
     * @param apiKeyId API key ID (partial or full)
     * @param userId User ID filter
     * @param orgId Organisation ID filter
     * @returns ApiKeyUsageResponse Usage count
     * @throws ApiError
     */
    public static fetchApiKeyUsage(
        xTenantId: string,
        date: string,
        apiKeyId?: string,
        userId?: string,
        orgId?: string,
    ): CancelablePromise<ApiKeyUsageResponse> {
        return __request(OpenAPI, {
            method: 'GET',
            url: '/usage',
            headers: {
                'X-Tenant-ID': xTenantId,
            },
            query: {
                'api_key_id': apiKeyId,
                'user_id': userId,
                'org_id': orgId,
                'date': date,
            },
        });
    }
    /**
     * @deprecated
     * DEPRECATED: Validate personal API key
     * DEPRECATED — Use POST /validate?key_type=personal instead. Validates a personal (user-scoped) API key.
     *
     * Requires `X-Tenant-ID` header for tenant resolution.
     * @param xTenantId Tenant identifier. Routes the request to the correct tenant context for user lookup and authentication.
     * @param requestBody
     * @returns PersonalApiKeyValidationResponse Personal API key is valid
     * @throws ApiError
     */
    public static validatePersonalApiKey(
        xTenantId: string,
        requestBody: ValidateApiKeyRequest,
    ): CancelablePromise<PersonalApiKeyValidationResponse> {
        return __request(OpenAPI, {
            method: 'POST',
            url: '/validate/personal',
            headers: {
                'X-Tenant-ID': xTenantId,
            },
            body: requestBody,
            mediaType: 'application/json',
            errors: {
                401: `Invalid or not a personal API key`,
            },
        });
    }
    /**
     * Fetch archived (revoked/expired) API keys
     * Returns paginated list of archived (revoked or expired) API keys.
     * SaaS customers can filter by their own users/orgs.
     *
     *
     * Requires `X-Tenant-ID` header for tenant resolution.
     * @param xTenantId Tenant identifier. Routes the request to the correct tenant context for user lookup and authentication.
     * @param userId Filter by API key owner user ID
     * @param orgId Filter by API key owner organisation ID
     * @param pageSize
     * @param pageNumber
     * @returns ArchivedApiKeyListResponse Archived API keys
     * @throws ApiError
     */
    public static fetchArchivedApiKeys(
        xTenantId: string,
        userId?: string,
        orgId?: string,
        pageSize: number = 10,
        pageNumber?: number,
    ): CancelablePromise<ArchivedApiKeyListResponse> {
        return __request(OpenAPI, {
            method: 'GET',
            url: '/archived',
            headers: {
                'X-Tenant-ID': xTenantId,
            },
            query: {
                'user_id': userId,
                'org_id': orgId,
                'page_size': pageSize,
                'page_number': pageNumber,
            },
        });
    }
    /**
     * Fetch archived API key details
     * Returns metadata for an archived (revoked/expired) API key.
     *
     * Requires `X-Tenant-ID` header for tenant resolution.
     * @param xTenantId Tenant identifier. Routes the request to the correct tenant context for user lookup and authentication.
     * @param keyId
     * @returns ArchivedApiKey Archived API key details
     * @throws ApiError
     */
    public static fetchArchivedApiKey(
        xTenantId: string,
        keyId: string,
    ): CancelablePromise<ArchivedApiKey> {
        return __request(OpenAPI, {
            method: 'GET',
            url: '/archived/{key_id}',
            path: {
                'key_id': keyId,
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
     * Import API keys from external system
     * Bulk imports API keys from another system. Each key is created with its
     * original secret hash, metadata, and expiration. Platform admin only.
     *
     *
     * Requires `X-Tenant-ID` header for tenant resolution.
     * @param xTenantId Tenant identifier. Routes the request to the correct tenant context for user lookup and authentication.
     * @param requestBody
     * @returns ImportApiKeysResponse API keys imported
     * @throws ApiError
     */
    public static importApiKeys(
        xTenantId: string,
        requestBody: ImportApiKeysRequest,
    ): CancelablePromise<ImportApiKeysResponse> {
        return __request(OpenAPI, {
            method: 'POST',
            url: '/import',
            headers: {
                'X-Tenant-ID': xTenantId,
            },
            body: requestBody,
            mediaType: 'application/json',
            errors: {
                400: `Invalid import data`,
            },
        });
    }
    /**
     * @deprecated
     * DEPRECATED: Validate organisation API key
     * DEPRECATED — Use POST /validate?key_type=org instead. Validates an organisation-scoped API key.
     *
     * Requires `X-Tenant-ID` header for tenant resolution.
     * @param xTenantId Tenant identifier. Routes the request to the correct tenant context for user lookup and authentication.
     * @param requestBody
     * @returns OrgApiKeyValidationResponse Organisation API key is valid
     * @throws ApiError
     */
    public static validateOrgApiKey(
        xTenantId: string,
        requestBody: ValidateApiKeyRequest,
    ): CancelablePromise<OrgApiKeyValidationResponse> {
        return __request(OpenAPI, {
            method: 'POST',
            url: '/validate/org',
            headers: {
                'X-Tenant-ID': xTenantId,
            },
            body: requestBody,
            mediaType: 'application/json',
            errors: {
                401: `Invalid or not an organisation API key`,
            },
        });
    }
}
