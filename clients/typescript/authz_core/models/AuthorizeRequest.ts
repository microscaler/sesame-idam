/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export type AuthorizeRequest = {
    /**
     * Principal's user ID
     */
    user_id: string;
    /**
     * Action to check (e.g., "read", "write", "delete", "approve")
     */
    action: string;
    /**
     * Resource identifier (e.g., "accounting:invoices", "users:123")
     */
    resource: string;
    /**
     * Application scope
     */
    app_id?: string | null;
    /**
     * Organisation scope
     */
    org_id?: string | null;
    /**
     * Tenant scope
     */
    tenant_id?: string | null;
    /**
     * Additional context for ABAC evaluation (e.g., resource attributes)
     */
    context?: Record<string, any> | null;
};

