/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export type ScimUser = {
    /**
     * SCIM schema URIs
     */
    schemas?: Array<string>;
    /**
     * SCIM user ID (maps to user_id in sesame-idam)
     */
    id: string;
    /**
     * Unique identifier for the user in the org
     */
    userName: string;
    /**
     * User name components
     */
    name: {
        familyName?: string;
        givenName?: string;
    };
    /**
     * Email addresses for the user
     */
    emails: Array<{
        value?: string;
        type?: 'primary' | 'work';
        primary?: boolean;
    }>;
    /**
     * Whether the user is active
     */
    active?: boolean;
    /**
     * SCIM roles mapped to sesame-idam org roles
     */
    roles?: Array<string>;
};

