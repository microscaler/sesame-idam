/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
import type { ApiKeyValidationResponse } from './ApiKeyValidationResponse';
export type OrgApiKeyValidationResponse = (ApiKeyValidationResponse & {
    /**
     * Always true — confirms this is an organisation-scoped key
     */
    is_org_scoped: boolean;
});

