/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export type Org = {
    /**
     * Organisation ID
     */
    id: string;
    /**
     * Organisation name
     */
    name: string;
    /**
     * Organisation slug
     */
    slug: string;
    /**
     * Organisation logo URL
     */
    logo_url?: string;
    /**
     * Primary organisation domain
     */
    domain?: string | null;
    /**
     * Additional domains for auto-join
     */
    domains?: Array<string> | null;
    /**
     * Whether domain-based auto-join is enabled for members
     */
    domain_auto_join?: boolean;
    /**
     * Whether signups are restricted to the organisation's domain(s) only
     */
    domain_restrict?: boolean;
    /**
     * Whether password rotation is enforced for all org users
     */
    password_rotation_enabled?: boolean;
    /**
     * Number of past passwords remembered to prevent reuse
     */
    password_rotation_history_size?: number;
    /**
     * Password expiry period in seconds (default 30 days)
     */
    password_rotation_period?: number;
    /**
     * Maximum number of users allowed (seat limit). null means unlimited.
     */
    max_users?: number | null;
    /**
     * Legacy organisation ID from a previous auth system
     */
    legacy_org_id?: string | null;
    /**
     * Free-form organisation metadata
     */
    metadata?: Record<string, any> | null;
    /**
     * Whether SAML SSO is configured for this org
     */
    is_saml_configured?: boolean;
    /**
     * Whether SAML is in test mode
     */
    is_saml_in_test_mode?: boolean;
    /**
     * Whether the org can configure SAML SSO
     */
    can_setup_saml?: boolean;
    /**
     * Whether the org uses isolated SAML (separate identity pool)
     */
    isolated?: boolean;
    /**
     * SAML trust level — AlwaysTrust, NeverTrust, TrustForDomain
     */
    sso_trust_level?: string | null;
    created_at: string;
    updated_at?: string;
};

