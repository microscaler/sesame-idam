/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export type ImpersonateRequest = {
    /**
     * Admin user performing the impersonation
     */
    actor_user_id: string;
    /**
     * Audit reason for impersonation
     */
    reason?: ImpersonateRequest.reason;
    /**
     * The target user to impersonate. This replaces the path parameter for security (prevents leaking user_id in access logs/CDN keys)
     */
    user_id: string;
};
export namespace ImpersonateRequest {
    /**
     * Audit reason for impersonation
     */
    export enum reason {
        DEBUG = 'debug',
        SUPPORT = 'support',
    }
}

