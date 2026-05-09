/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export type RegisterRequest = {
    email: string;
    password: string;
    /**
     * Legal-entity customer
     */
    organization_id?: string | null;
    /**
     * Tenant for multi-tenant org
     */
    tenant_id?: string | null;
    name?: string;
};

