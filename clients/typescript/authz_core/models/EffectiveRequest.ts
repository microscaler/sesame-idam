/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export type EffectiveRequest = {
    /**
     * Principal's user ID
     */
    user_id: string;
    /**
     * Application ID
     */
    app_id: string;
    /**
     * Organisation scope
     */
    org_id?: string | null;
    /**
     * Tenant scope
     */
    tenant_id: string;
    /**
     * Include roles/permissions inherited from parent scopes
     */
    include_inherited?: boolean;
};

