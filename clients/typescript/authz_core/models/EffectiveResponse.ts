/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export type EffectiveResponse = {
    /**
     * Principal's user ID
     */
    user_id: string;
    roles: Array<{
        /**
         * Role name
         */
        role?: string;
        /**
         * Application ID
         */
        app_id?: string;
        /**
         * Organisation scope
         */
        org_id?: string | null;
        /**
         * Whether this role was inherited
         */
        inherited?: boolean;
    }>;
    /**
     * Flat list of all permission codes (e.g., "accounting:invoices:write")
     */
    permissions: Array<string>;
    /**
     * ABAC attributes set on this principal
     */
    attributes?: Record<string, string>;
};

