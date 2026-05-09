/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
/**
 * Partial update for organisation settings. All fields are optional.
 */
export type UpdateOrgRequest = {
    /**
     * Organisation name
     */
    name?: string | null;
    /**
     * Organisation slug (URL-safe identifier)
     */
    slug?: string | null;
    /**
     * Organisation logo URL
     */
    logo_url?: string | null;
    /**
     * Free-form organisation metadata
     */
    metadata?: Record<string, any> | null;
    /**
     * Primary organisation domain
     */
    domain?: string | null;
    /**
     * Additional domains for auto-join
     */
    domains?: Array<string> | null;
    /**
     * Toggle domain-based auto-join for members
     */
    domain_auto_join?: boolean | null;
    /**
     * Restrict signups to the organisation's domain(s) only
     */
    domain_restrict?: boolean | null;
    /**
     * Enable or disable password rotation enforcement
     */
    password_rotation_enabled?: boolean | null;
    /**
     * Number of past passwords to remember (1-24)
     */
    password_rotation_history_size?: number | null;
    /**
     * Password expiry period in seconds (minimum 1 hour)
     */
    password_rotation_period?: number | null;
    /**
     * Seat limit for the organisation. null means unlimited.
     */
    max_users?: number | null;
    /**
     * Legacy organisation ID
     */
    legacy_org_id?: string | null;
    /**
     * Toggle SAML configuration (write-only flag)
     */
    is_saml_configured?: boolean | null;
    /**
     * Migrate to isolated SAML mode
     */
    isolated?: boolean | null;
};

