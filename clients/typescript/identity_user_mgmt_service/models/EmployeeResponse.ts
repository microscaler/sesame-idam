/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export type EmployeeResponse = {
    user_id?: string;
    email?: string;
    first_name?: string | null;
    last_name?: string | null;
    username?: string | null;
    picture_url?: string | null;
    org_id_to_org_info?: Record<string, {
        org_id?: string;
        org_name?: string;
        user_role?: string;
        user_permissions?: Array<string>;
    }>;
};

