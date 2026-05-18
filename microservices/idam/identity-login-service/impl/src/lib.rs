// Clippy configuration for production code quality
// Allow clippy lints that come from derive macros (LifeModel/LifeRecord) and
// build script output (entity_registry.rs) — these are auto-generated and
// cannot be controlled from hand-written code.
#![allow(clippy::pub_underscore_fields)]         // LifeRecord generates pub _field fields
#![allow(clippy::needless_raw_string_hashes)]    // build script generates r#"..."#
#![allow(clippy::must_use_candidate)]            // build script generates must_use functions
#![allow(clippy::missing_errors_doc)]            // build script docs are auto-generated
#![allow(clippy::uninlined_format_args)]         // build script format strings
#![allow(clippy::missing_panics_doc)]            // build script panic docs are auto-generated
//! Identity login service — library target for migrator access.

pub mod models;
