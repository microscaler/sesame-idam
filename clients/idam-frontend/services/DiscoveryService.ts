/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
import type { JWKS } from '../models/JWKS';
import type { OpenIDConfiguration } from '../models/OpenIDConfiguration';
import type { CancelablePromise } from '../core/CancelablePromise';
import { OpenAPI } from '../core/OpenAPI';
import { request as __request } from '../core/request';
export class DiscoveryService {
    /**
     * OIDC discovery
     * @returns OpenIDConfiguration
     * @throws ApiError
     */
    public static openidConfiguration(): CancelablePromise<OpenIDConfiguration> {
        return __request(OpenAPI, {
            method: 'GET',
            url: '/.well-known/openid-configuration',
        });
    }
    /**
     * JWKS for JWT verification
     * @returns JWKS
     * @throws ApiError
     */
    public static jwks(): CancelablePromise<JWKS> {
        return __request(OpenAPI, {
            method: 'GET',
            url: '/.well-known/jwks.json',
        });
    }
}
