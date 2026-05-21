// BDD test hub for identity-login-service

pub mod common;

pub mod bdd {
    pub mod jwt_ttl;
    pub mod jwt_validation;
    pub mod pii_entitlements;
    pub mod smoke;
}
