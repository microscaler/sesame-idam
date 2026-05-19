// BDD test hub for authz-core

pub mod common;

pub mod bdd {
    pub mod all_endpoints;
    pub mod authorize;
    pub mod jwt_validation;
    pub mod principal_effective;
    pub mod set_principal_attribute;
}
