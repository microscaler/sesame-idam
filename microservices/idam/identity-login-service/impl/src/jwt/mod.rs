/// JWT access token TTL configuration with role-based tiers.
///
/// All token types use 5-minute (300s) TTL after F-010 alignment.
pub mod ttl;

pub use ttl::{validate_minimum_ttl, validate_refresh_exceeds_access, TtlConfig};
