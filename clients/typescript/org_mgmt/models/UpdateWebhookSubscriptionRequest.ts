/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export type UpdateWebhookSubscriptionRequest = {
    /**
     * New endpoint URL
     */
    endpoint_url?: string | null;
    /**
     * Updated list of subscribed events
     */
    events?: Array<string> | null;
    /**
     * Optional new shared secret for signing
     */
    secret?: string | null;
    /**
     * Toggle webhook enabled status
     */
    enabled?: boolean | null;
    /**
     * Updated metadata
     */
    metadata?: Record<string, any> | null;
};

