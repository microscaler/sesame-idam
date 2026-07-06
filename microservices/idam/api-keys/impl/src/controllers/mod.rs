// The `#[handler]` macro requires `handle(req: TypedHandlerRequest<Request>)`
// by value — suppress clippy::needless_pass_by_value for all controllers.
#![allow(clippy::needless_pass_by_value)]
//! API key management controllers.
//!
//! Only implemented controllers are declared here and registered in main.rs
//! via the Register & Overwrite pattern (gen stubs serve the rest). The
//! remaining controller files in this directory are earlier echo drafts —
//! re-enable them one at a time as they are implemented against the real
//! persistence layer.

pub mod create_api_key;
pub mod validate_api_key;
