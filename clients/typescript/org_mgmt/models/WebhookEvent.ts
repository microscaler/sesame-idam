/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export type WebhookEvent = {
    /**
     * Unique ID for this webhook event
     */
    event_id: string;
    /**
     * The event type that triggered this notification
     */
    event_type: string;
    /**
     * Organisation ID this event relates to
     */
    org_id: string;
    /**
     * When the event occurred
     */
    timestamp: string;
    /**
     * Event-specific payload data
     */
    data: Record<string, any>;
    /**
     * Number of delivery attempts for this event
     */
    delivery_attempts?: number;
};

