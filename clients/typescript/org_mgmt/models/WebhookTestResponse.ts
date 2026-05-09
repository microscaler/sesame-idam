/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export type WebhookTestResponse = {
    /**
     * Whether the test delivery was sent successfully
     */
    success?: boolean;
    /**
     * The endpoint that received the test event
     */
    endpoint_url?: string;
    /**
     * HTTP status code returned by the endpoint
     */
    delivery_status?: number | null;
    /**
     * Status message
     */
    message?: string;
};

