/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export type CreateWebhookSubscriptionRequest = {
    /**
     * The URL to receive webhook events
     */
    endpoint_url: string;
    /**
     * List of events to subscribe to
     */
    events: Array<'org.created' | 'org.updated' | 'org.deleted' | 'user.joined' | 'user.left' | 'user.role_changed' | 'user.enabled' | 'user.disabled' | 'invite.created' | 'invite.accepted' | 'invite.revoked' | 'role_assigned' | 'role_removed' | 'domain_added' | 'domain_removed' | 'saml_configured' | 'saml_disabled'>;
    /**
     * Optional shared secret for signing webhook payloads (HMAC-SHA256)
     */
    secret?: string | null;
    /**
     * Whether this webhook is currently active
     */
    enabled?: boolean;
    /**
     * Custom key-value metadata
     */
    metadata?: Record<string, any> | null;
};

