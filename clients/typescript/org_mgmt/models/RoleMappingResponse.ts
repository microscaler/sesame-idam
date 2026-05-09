/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export type RoleMappingResponse = {
    org_id?: string;
    /**
     * Plan-based role mapping name
     */
    mapping_name?: string;
    /**
     * Roles automatically assigned to org members
     */
    assigned_roles?: Array<string>;
    subscribed_at?: string;
};

