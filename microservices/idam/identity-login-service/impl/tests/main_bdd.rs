// BDD test hub for identity-login-service

pub mod common;

pub mod bdd {
    pub mod account_first_onboarding;
    pub mod auth_flow;
    pub mod authz_enrichment;
    pub mod jwt_ttl;
    pub mod jwt_validation;
    pub mod pii_entitlements;
    pub mod smoke;
    pub mod token_lifecycle;
}
