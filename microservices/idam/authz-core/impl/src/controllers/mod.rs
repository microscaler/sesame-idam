/// Authorization controllers for Sesame-IDAM.
///
/// Each controller corresponds to a single API endpoint and is wired
/// into the BRRTRouter dispatcher. Controllers audit every request
/// via the global `EMITTER`, then delegate to the authz service layer.

pub mod authorize;
pub mod principal_effective;
pub mod set_principal_attribute;
pub mod assign_principal_role;
pub mod revoke_principal_role;