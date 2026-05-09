/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
import type { ApiKeyValidationResponse } from './ApiKeyValidationResponse';
export type PersonalApiKeyValidationResponse = (ApiKeyValidationResponse & {
    /**
     * Always true — confirms this is a personal (user-scoped) key
     */
    is_personal: boolean;
});

