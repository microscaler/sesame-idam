/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
/**
 * SCIM 2.0 Error Response (RFC 7644 Section 3.7)
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
    scimType?: ScimError.scimType;
};
export namespace ScimError {
    /**
     * SCIM error type per RFC 7643 Section 3.5.2
     */
    export enum scimType {
        INVALID_FILTER = 'invalidFilter',
        UNIQUENESS = 'uniqueness',
        VALUE = 'value',
        MUTABILITY = 'mutability',
        INVALID_PATH = 'invalidPath',
        NO_TARGET = 'noTarget',
        SENSITIVE = 'sensitive',
        TOO_MANY = 'tooMany',
    }
}

