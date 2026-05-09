/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
/**
 * An MCP agent registered for use with Sesame-IDAM
 */
export type McpAgent = {
    /**
     * Unique agent identifier
     */
    agent_id: string;
    /**
     * Human-readable agent name
     */
    name: string;
    /**
     * Agent description
     */
    description?: string | null;
    /**
     * Creation timestamp
     */
    created_at: string;
    /**
     * Last update timestamp
     */
    updated_at: string;
    /**
     * Whether the agent is active
     */
    active: boolean;
};

