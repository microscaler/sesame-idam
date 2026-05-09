/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
import type { WebhookSubscriptionListResponse } from '../models/WebhookSubscriptionListResponse';
import type { WebhookTestResponse } from '../models/WebhookTestResponse';
import type { CancelablePromise } from '../core/CancelablePromise';
import { OpenAPI } from '../core/OpenAPI';
import { request as __request } from '../core/request';
export class WebhooksService {
    /**
     * Fetch organisation webhook subscriptions
     * Returns all active webhook subscriptions for this organisation.
     *
     * Requires `X-Tenant-ID` header for tenant resolution.
     * @param xTenantId Tenant identifier. Routes the request to the correct tenant context for user lookup and authentication.
     * @param orgId
     * @param page Page number for pagination
     * @param limit Number of items per page (max 100)
     * @returns WebhookSubscriptionListResponse Webhook subscriptions
     * @throws ApiError
     */
    public static fetchWebhookSubscriptions(
        xTenantId: string,
        orgId: string,
        page: number = 1,
        limit: number = 20,
    ): CancelablePromise<WebhookSubscriptionListResponse> {
        return __request(OpenAPI, {
            method: 'GET',
            url: '/{org_id}/webhooks',
            path: {
                'org_id': orgId,
            },
            headers: {
                'X-Tenant-ID': xTenantId,
            },
            query: {
                'page': page,
                'limit': limit,
            },
        });
    }
    /**
     * Delete webhook subscription
     * Deletes a webhook subscription. No more events will be sent to this endpoint.
     * Irreversible - use disable to temporarily stop deliveries.
     *
     *
     * Requires `X-Tenant-ID` header for tenant resolution.
     * @param xTenantId Tenant identifier. Routes the request to the correct tenant context for user lookup and authentication.
     * @param orgId
     * @param subscriptionId
     * @returns void
     * @throws ApiError
     */
    public static deleteWebhookSubscription(
        xTenantId: string,
        orgId: string,
        subscriptionId: string,
    ): CancelablePromise<void> {
        return __request(OpenAPI, {
            method: 'DELETE',
            url: '/{org_id}/webhooks/{subscription_id}',
            path: {
                'org_id': orgId,
                'subscription_id': subscriptionId,
            },
            headers: {
                'X-Tenant-ID': xTenantId,
            },
            errors: {
                404: `Not found`,
            },
        });
    }
    /**
     * Test webhook delivery
     * Sends a test event to the webhook endpoint to verify connectivity
     * and payload format. Useful for debugging webhook configuration.
     *
     *
     * Requires `X-Tenant-ID` header for tenant resolution.
     * @param xTenantId Tenant identifier. Routes the request to the correct tenant context for user lookup and authentication.
     * @param orgId
     * @param subscriptionId
     * @returns WebhookTestResponse Test delivery sent
     * @throws ApiError
     */
    public static testWebhookDelivery(
        xTenantId: string,
        orgId: string,
        subscriptionId: string,
    ): CancelablePromise<WebhookTestResponse> {
        return __request(OpenAPI, {
            method: 'POST',
            url: '/{org_id}/webhooks/{subscription_id}/test',
            path: {
                'org_id': orgId,
                'subscription_id': subscriptionId,
            },
            headers: {
                'X-Tenant-ID': xTenantId,
            },
            errors: {
                404: `Not found`,
            },
        });
    }
}
