/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
import type { ScimUser } from './ScimUser';
/**
 * Property examples:
 *  * - `schemas`: `["urn:ietf:params:scim:api:messages:2.0:ListResponse"]`
 */

export type ScimUserListResponse = {
    totalResults: number;
    itemsPerPage: number;
    startIndex: number;
    schemas?: Array<string>;
    Resources: Array<ScimUser>;
};

