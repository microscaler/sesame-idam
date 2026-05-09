/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export type SetPrincipalAttributeRequest = {
    /**
     * Principal's user ID
     */
    user_id: string;
    /**
     * Attribute key (e.g., "department", "clearance_level", "region")
     */
    key: string;
    /**
     * Attribute value
     */
    value: string;
    /**
     * Organisation scope (optional — unset for user-level attributes)
     */
    org_id?: string | null;
    /**
     * Tenant scope
     */
    tenant_id: string;
};

