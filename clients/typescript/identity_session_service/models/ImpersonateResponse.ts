/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export type ImpersonateResponse = {
    /**
     * ID of the user being impersonated
     */
    impersonated_user_id: string;
    /**
     * Access token for impersonated user
     */
    access_token: string;
    /**
     * Refresh token for impersonated user
     */
    refresh_token: string;
    /**
     * Admin user ID (for restore)
     */
    original_user_id: string;
};

