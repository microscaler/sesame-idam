/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export type ScimUserUpdateRequest = {
    schemas?: Array<string>;
    userName?: string;
    name?: {
        familyName?: string;
        givenName?: string;
    };
    emails?: Array<{
        value?: string;
        type?: string;
        primary?: boolean;
    }>;
    active?: boolean;
    roles?: Array<string>;
};

