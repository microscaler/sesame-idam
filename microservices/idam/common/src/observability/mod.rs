//! Observability helpers shared across Sesame services.

pub mod redaction;

pub use redaction::{
    assert_no_redacted_fields, redact_sensitive_object, redacted_field_names, DEFAULT_REDACTED_FIELDS,
};
