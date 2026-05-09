/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export type ScimGroup = {
    id: string;
    /**
     * Group name
     */
    name: string;
    /**
     * Group description
     */
    description?: string;
    members: Array<{
        user_id?: string;
        email?: string;
    }>;
    created_at?: string;
    updated_at?: string;
};

