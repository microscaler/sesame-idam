/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export type CreateOrgRequest = {
    /**
     * Organisation name
     */
    name: string;
    /**
     * Organisation slug (URL-safe identifier)
     */
    slug?: string;
    /**
     * Organisation logo URL
     */
    logo_url?: string;
    /**
     * Primary organisation domain
     */
    domain?: string;
    /**
     * Additional domains for auto-join
     */
    domains?: Array<string>;
    /**
     * Enable domain-based auto-join for members
     */
    domain_auto_join?: boolean;
    /**
     * Restrict signups to the organisation's domain(s) only
     */
    domain_restrict?: boolean;
    /**
     * Enforce password rotation for all org users
     */
    password_rotation_enabled?: boolean;
    /**
     * Number of past passwords to remember (to prevent reuse)
     */
    password_rotation_history_size?: number;
    /**
     * Password expiry period in seconds (default 30 days)
     */
    password_rotation_period?: number;
    /**
     * Maximum number of users allowed in the organisation (seat limit). null means unlimited.
     */
    max_users?: number | null;
    /**
     * Legacy organisation ID from a previous auth system
     */
    legacy_org_id?: string | null;
    /**
     * Free-form organisation metadata
     */
    metadata?: Record<string, any>;
};

