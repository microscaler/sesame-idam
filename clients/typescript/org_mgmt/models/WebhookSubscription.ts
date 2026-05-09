/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export type WebhookSubscription = {
    /**
     * Unique ID of the webhook subscription
     */
    subscription_id?: string;
    /**
     * Organisation ID this subscription belongs to
     */
    org_id?: string;
    /**
     * The URL to receive webhook events
     */
    endpoint_url?: string;
    /**
     * List of subscribed events
     */
    events?: Array<string>;
    /**
     * Whether a signing secret is configured (never returns the actual secret)
     */
    secret_present?: boolean;
    /**
     * Whether this webhook is currently active
     */
    enabled?: boolean;
    /**
     * Timestamp of the last successful delivery
     */
    last_delivery_at?: string | null;
    /**
     * HTTP status of the last delivery attempt
     */
    last_delivery_status?: string | null;
    /**
     * Total number of delivery attempts
     */
    total_deliveries?: number;
    /**
     * Number of failed delivery attempts
     */
    failed_deliveries?: number;
    /**
     * When the subscription was created
     */
    created_at?: string;
    /**
     * When the subscription was last updated
     */
    updated_at?: string;
};

