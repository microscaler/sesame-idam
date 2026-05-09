/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export type PendingInvitesResponse = {
    items: Array<{
        id?: string;
        email?: string;
        role?: string;
        invited_by?: string;
        created_at?: string;
    }>;
    total: number;
    page: number;
    page_size: number;
};

