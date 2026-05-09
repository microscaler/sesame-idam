/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
/**
 * Property examples:
 *  * - `token_type`: `"Bearer"`
 */

export type McpTokenResponse = {
    /**
     * MCP access token
     */
    access_token: string;
    token_type?: 'Bearer';
    /**
     * Token lifetime in seconds
     */
    expires_in?: number;
    /**
     * Granted scopes
     */
    scope?: string;
};

