/// JWT access token TTL configuration with role-based tiers.
pub mod ttl;

pub use ttl::{validate_minimum_ttl, validate_refresh_exceeds_access, TtlConfig};
