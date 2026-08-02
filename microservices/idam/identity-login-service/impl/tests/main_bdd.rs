// BDD test hub for identity-login-service

pub mod common;

pub mod bdd {
    pub mod account_first_onboarding;
    pub mod account_lockout;
    pub mod auth_flow;
    pub mod authz_enrichment;
    pub mod east_west_live_api;
    pub mod email_round_trip;
    pub mod jwt_ttl;
    pub mod jwt_validation;
    pub mod logout_revocation;
    pub mod north_live_api;
    pub mod oidc_conformance;
    pub mod oidc_interactive;
    pub mod oidc_live_api;
    pub mod oidc_protocol;
    pub mod otp_caps;
    pub mod password_reset;
    pub mod phone_otp;
    pub mod pii_entitlements;
    pub mod platform_tenant_admin;
    pub mod pre_oidc_stubs;
    pub mod session_handoff;
    pub mod signup_validate;
    pub mod sms_magic_link;
    pub mod smoke;
    pub mod social_login_flow;
    pub mod tenant_claims;
    pub mod tenant_sms_custody;
    pub mod token_lifecycle;
}
