/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export type ScimUserCreateRequest = {
    schemas?: Array<string>;
    /**
     * User email/identifier
     */
    userName: string;
    name: {
        familyName: string;
        givenName: string;
    };
    emails?: Array<{
        value?: string;
        type?: 'primary' | 'work';
        primary?: boolean;
    }>;
    /**
     * Whether user is active
     */
    active?: boolean;
    /**
     * SCIM roles mapped to sesame-idam org roles
     */
    roles?: Array<string>;
};

