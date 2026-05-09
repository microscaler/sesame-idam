/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export type MigrateUserRequest = {
    email: string;
    email_confirmed?: boolean;
    first_name?: string;
    last_name?: string;
    username?: string;
    picture_url?: string;
    legacy_user_id?: string;
    extra_properties?: Record<string, any>;
    hash?: string;
    salt?: string;
    /**
     * For SaaS: required, their own org. For platform: optional.
     *
     */
    org_id?: string | null;
};

