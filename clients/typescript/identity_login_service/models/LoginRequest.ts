/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export type LoginRequest = {
    /**
     * Primary email address
     */
    email: string;
    /**
     * User's password
     */
    password: string;
    /**
     * Optional org hint for multi-tenant login
     */
    organization_id?: string | null;
};

