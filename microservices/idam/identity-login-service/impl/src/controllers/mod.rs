// The `#[handler]` macro requires `handle(req: TypedHandlerRequest<Request>)`
// by value — suppress clippy::needless_pass_by_value for all controllers.
#![allow(clippy::needless_pass_by_value)]
//! Controller handlers for Authentication.
//!
//! Only implemented controllers are declared here and registered in main.rs
//! via the Register & Overwrite pattern (ADR 0001 in hauliage): gen stubs are
//! registered first, then these implementations overwrite their routes.
//!
//! The remaining controller files in this directory (OTP, magic link, social
//! OAuth, token exchange, ...) are earlier drafts that predate the current
//! gen types — they are re-enabled here one at a time as they are implemented
//! against the real persistence layer.

pub mod auth_login;
pub mod auth_logout;
pub mod auth_register;
