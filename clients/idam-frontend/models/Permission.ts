/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export type Permission = {
    id: string;
    /**
     * Permission name (e.g. "read:docs")
     */
    name: string;
    /**
     * Permission description
     */
    description?: string;
    /**
     * Owning application ID
     */
    application_id: string;
    created_at: string;
    updated_at?: string;
};

