//! Shared token versioning primitives for Sesame-IDAM.
//!
//! Provides the `VersionBumpEvent` type, `VersionBumpPublisher` for broadcasting
//! version bumps via Redis pub/sub, `VersionBumpSubscriber` for consuming
//! them with a local in-memory cache, and `VersionStore` for atomic
//! INCR/GET operations on `authz_ver` keys.

pub mod events;
pub mod publisher;
pub mod subscriber;
pub mod version_store;

pub use events::VersionBumpEvent;
pub use publisher::VersionBumpPublisher;
pub use subscriber::VersionBumpSubscriber;
pub use version_store::{VersionStore, VersionStoreConfig, subject_key, tenant_key};
