/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
/**
 * SCIM 2.0 Error Response (RFC 7644 Section 3.7)
 */
/**
 * Example usage:
 * ```typescript
 * const example: ScimError = {
  "schemas": [
    "urn:ietf:params:scim:api:messages:2.0:Error"
  ],
  "detail": "The requested attribute is not supported",
  "status": "400",
  "scimType": "invalidFilter"
};
 * ```
 */

export type ScimError = {
    schemas: Array<string>;
    /**
     * Human-readable error message
     */
    detail: string;
    /**
     * HTTP status code as string (e.g., '400')
     */
    status: string;
    /**
     * SCIM error type per RFC 7643 Section 3.5.2
     */
    scimType?: 'invalidFilter' | 'uniqueness' | 'value' | 'mutability' | 'invalidPath' | 'noTarget' | 'sensitive' | 'tooMany';
};

