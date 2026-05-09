/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export type AuthorizeResponse = {
    /**
     * Whether the principal is authorized for this action
     */
    allowed: boolean;
    /**
     * Reason for denial (null when allowed).
     * Examples: "no_matching_role", "insufficient_permissions", "resource_not_owned"
     *
     */
    reason?: string | null;
    /**
     * Roles that contributed to the decision (for audit logging)
     */
    roles_matched?: Array<string> | null;
    /**
     * Permission codes that granted access
     */
    permissions_used?: Array<string> | null;
};

