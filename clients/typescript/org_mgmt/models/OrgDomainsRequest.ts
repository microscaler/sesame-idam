/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export type OrgDomainsRequest = {
    /**
     * Primary organisation domain
     */
    primary_domain?: string;
    /**
     * Enable domain-based auto-join
     */
    auto_join_domain?: boolean;
    /**
     * Additional domains for auto-join
     */
    extra_domains?: Array<string>;
};

