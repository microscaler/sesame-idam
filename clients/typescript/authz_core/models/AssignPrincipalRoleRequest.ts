/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export type AssignPrincipalRoleRequest = {
    /**
     * Principal's user ID (the `sub` claim)
     */
    user_id: string;
    /**
     * Role identifier (e.g., "admin", "editor", "viewer")
     */
    role: string;
    /**
     * Application ID that defines this role
     */
    app_id: string;
    /**
     * Organisation scope (for multi-tenant orgs)
     */
    org_id?: string | null;
    /**
     * Tenant scope (for multi-tenant orgs)
     */
    tenant_id: string;
    /**
     * Optional role expiration time
     */
    expires_at?: string | null;
};

