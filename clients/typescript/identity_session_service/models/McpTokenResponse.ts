/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export type McpTokenResponse = {
    /**
     * MCP access token
     */
    access_token: string;
    token_type?: McpTokenResponse.token_type;
    /**
     * Token lifetime in seconds
     */
    expires_in?: number;
    /**
     * Granted scopes
     */
    scope?: string;
};
export namespace McpTokenResponse {
    export enum token_type {
        BEARER = 'Bearer',
    }
}

