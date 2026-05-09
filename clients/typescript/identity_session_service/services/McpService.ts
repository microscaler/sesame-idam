/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
import type { McpAgent } from '../models/McpAgent';
import type { McpAgentCreateResponse } from '../models/McpAgentCreateResponse';
import type { McpAgentListResponse } from '../models/McpAgentListResponse';
import type { McpTokenRequest } from '../models/McpTokenRequest';
import type { McpTokenResponse } from '../models/McpTokenResponse';
import type { McpValidateRequest } from '../models/McpValidateRequest';
import type { McpValidateResponse } from '../models/McpValidateResponse';
import type { CancelablePromise } from '../core/CancelablePromise';
import { OpenAPI } from '../core/OpenAPI';
import { request as __request } from '../core/request';
export class McpService {
    /**
     * Issue MCP auth token
     * Authenticate an MCP agent and issue a token for tool access.
     *
     * Requires `X-Tenant-ID` header for tenant resolution.
     * @param xTenantId Tenant identifier. Routes the request to the correct tenant context for user lookup and authentication.
     * @param requestBody
     * @returns McpTokenResponse MCP token issued
     * @throws ApiError
     */
    public static mcpToken(
        xTenantId: string,
        requestBody: McpTokenRequest,
    ): CancelablePromise<McpTokenResponse> {
        return __request(OpenAPI, {
            method: 'POST',
            url: '/mcp/token',
            headers: {
                'X-Tenant-ID': xTenantId,
            },
            body: requestBody,
            mediaType: 'application/json',
            errors: {
                401: `Invalid agent credentials`,
            },
        });
    }
    /**
     * Validate MCP token
     * Validate an MCP token and return its claims.
     *
     * Requires `X-Tenant-ID` header for tenant resolution.
     * @param xTenantId Tenant identifier. Routes the request to the correct tenant context for user lookup and authentication.
     * @param requestBody
     * @returns McpValidateResponse Validation result
     * @throws ApiError
     */
    public static mcpValidate(
        xTenantId: string,
        requestBody: McpValidateRequest,
    ): CancelablePromise<McpValidateResponse> {
        return __request(OpenAPI, {
            method: 'POST',
            url: '/mcp/token/validate',
            headers: {
                'X-Tenant-ID': xTenantId,
            },
            body: requestBody,
            mediaType: 'application/json',
            errors: {
                401: `Invalid or expired token`,
            },
        });
    }
    /**
     * List agents
     * List all MCP agents for the authenticated org.
     *
     * Requires `X-Tenant-ID` header for tenant resolution.
     * @param xTenantId Tenant identifier. Routes the request to the correct tenant context for user lookup and authentication.
     * @returns McpAgentListResponse List of MCP agents
     * @throws ApiError
     */
    public static mcpListAgents(
        xTenantId: string,
    ): CancelablePromise<McpAgentListResponse> {
        return __request(OpenAPI, {
            method: 'GET',
            url: '/api/v1/platform/mcp/agents',
            headers: {
                'X-Tenant-ID': xTenantId,
            },
            errors: {
                401: `Unauthorized`,
            },
        });
    }
    /**
     * Create agent
     * Register a new MCP agent.
     *
     * Requires `X-Tenant-ID` header for tenant resolution.
     * @param xTenantId Tenant identifier. Routes the request to the correct tenant context for user lookup and authentication.
     * @param requestBody
     * @returns McpAgentCreateResponse Agent created successfully
     * @throws ApiError
     */
    public static mcpCreateAgent(
        xTenantId: string,
        requestBody: {
            /**
             * Human-readable agent name
             */
            name: string;
            /**
             * Agent description
             */
            description?: string | null;
        },
    ): CancelablePromise<McpAgentCreateResponse> {
        return __request(OpenAPI, {
            method: 'POST',
            url: '/api/v1/platform/mcp/agents',
            headers: {
                'X-Tenant-ID': xTenantId,
            },
            body: requestBody,
            mediaType: 'application/json',
            errors: {
                400: `Invalid request`,
                401: `Unauthorized`,
            },
        });
    }
    /**
     * Get agent
     * Requires `X-Tenant-ID` header for tenant resolution.
     * @param xTenantId Tenant identifier. Routes the request to the correct tenant context for user lookup and authentication.
     * @returns McpAgent Agent details
     * @throws ApiError
     */
    public static mcpGetAgent(
        xTenantId: string,
    ): CancelablePromise<McpAgent> {
        return __request(OpenAPI, {
            method: 'GET',
            url: '/api/v1/platform/mcp/agents/{agent_id}',
            headers: {
                'X-Tenant-ID': xTenantId,
            },
            errors: {
                404: `Agent not found`,
            },
        });
    }
    /**
     * Delete agent
     * Requires `X-Tenant-ID` header for tenant resolution.
     * @param xTenantId Tenant identifier. Routes the request to the correct tenant context for user lookup and authentication.
     * @returns void
     * @throws ApiError
     */
    public static mcpDeleteAgent(
        xTenantId: string,
    ): CancelablePromise<void> {
        return __request(OpenAPI, {
            method: 'DELETE',
            url: '/api/v1/platform/mcp/agents/{agent_id}',
            headers: {
                'X-Tenant-ID': xTenantId,
            },
            errors: {
                404: `Agent not found`,
            },
        });
    }
}
