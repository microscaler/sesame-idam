//! Sesame's own Prometheus series, and the text every service appends to BRRTRouter's `/metrics`.
//!
//! BRRTRouter renders the HTTP layer; `set_extra_prometheus` lets each service append more. This module appends
//! the token-status counters and everything registered in the `prometheus` crate's default registry: the series
//! below and the older ones registered with `register_*!` macros (for example `token_refresh_total`,
//! `refresh_reuse_detected_total`), which nothing rendered before.
//!
//! # Series
//!
//! | name | labels | what |
//! |------|--------|------|
//! | `sesame_security_events_total` | service, event, source, result, reason | every audit event (sign-in, refresh, revocation, key rotation…) |
//! | `sesame_kms_requests_total` | op, outcome | Cloud KMS calls (ID-token signing, public keys, MACs) |
//! | `sesame_kms_request_duration_seconds` | op | Cloud KMS call latency |
//! | `sesame_mail_send_total` | kind, outcome | SMTP hand-offs to the mail relay |
//! | `sesame_mail_send_duration_seconds` | — | SMTP hand-off latency |
//! | `sesame_dependency_errors_total` | dependency, component | a dependency failed and Sesame degraded or refused |
//!
//! Labels are fixed, low-cardinality values. Free text (error messages, identifiers) never becomes a label:
//! [`label`] maps anything that is not a short `snake_case` token to `other`.

use std::sync::LazyLock;

use prometheus::{
    register_histogram, register_histogram_vec, register_int_counter_vec, Encoder, Histogram,
    HistogramVec, IntCounterVec, TextEncoder,
};

/// Every audit event, by the emitting service, its type, the flow that produced it, its result and reason.
pub static SECURITY_EVENTS: LazyLock<IntCounterVec> = LazyLock::new(|| {
    register_int_counter_vec!(
        "sesame_security_events_total",
        "Audit events by service, event type, flow (source), result and reason",
        &["service", "event", "source", "result", "reason"]
    )
    .expect("sesame_security_events_total registers once")
});

/// Cloud KMS calls by operation and outcome.
pub static KMS_REQUESTS: LazyLock<IntCounterVec> = LazyLock::new(|| {
    register_int_counter_vec!(
        "sesame_kms_requests_total",
        "Cloud KMS requests by operation (asymmetricSign, macSign, publicKey, token) and outcome",
        &["op", "outcome"]
    )
    .expect("sesame_kms_requests_total registers once")
});

/// Cloud KMS latency by operation.
pub static KMS_DURATION: LazyLock<HistogramVec> = LazyLock::new(|| {
    register_histogram_vec!(
        "sesame_kms_request_duration_seconds",
        "Cloud KMS request latency by operation",
        &["op"],
        vec![0.01, 0.025, 0.05, 0.1, 0.2, 0.3, 0.5, 1.0, 2.0, 5.0]
    )
    .expect("sesame_kms_request_duration_seconds registers once")
});

/// SMTP hand-offs to the mail relay by outcome.
pub static MAIL_SENDS: LazyLock<IntCounterVec> = LazyLock::new(|| {
    register_int_counter_vec!(
        "sesame_mail_send_total",
        "Mail hand-offs to the SMTP relay by kind (otp, magic_link, password_reset, invite, owner_transfer, security_notice) and outcome (sent, refused, unreachable, error, suppressed: a reserved non-deliverable domain, not sent; captured: handed to the pre-production capture sink)",
        &["kind", "outcome"]
    )
    .expect("sesame_mail_send_total registers once")
});

/// SMTP hand-off latency.
pub static MAIL_DURATION: LazyLock<Histogram> = LazyLock::new(|| {
    register_histogram!(
        "sesame_mail_send_duration_seconds",
        "Time to hand a message to the SMTP relay",
        vec![0.05, 0.1, 0.25, 0.5, 1.0, 2.0, 5.0, 10.0, 30.0]
    )
    .expect("sesame_mail_send_duration_seconds registers once")
});

/// A dependency failed and Sesame degraded (or refused) because of it.
pub static DEPENDENCY_ERRORS: LazyLock<IntCounterVec> = LazyLock::new(|| {
    register_int_counter_vec!(
        "sesame_dependency_errors_total",
        "Dependency failures by dependency (redis, postgres, kms, smtp, authz-core) and component",
        &["dependency", "component"]
    )
    .expect("sesame_dependency_errors_total registers once")
});

/// Count a dependency failure and log it once, with the component that saw it.
pub fn dependency_error(
    dependency: &'static str,
    component: &'static str,
    error: &dyn std::fmt::Display,
) {
    DEPENDENCY_ERRORS
        .with_label_values(&[dependency, component])
        .inc();
    tracing::warn!(dependency, component, error = %error, "sesame_dependency_error");
}

/// A label value safe for Prometheus: a short `snake_case`/`kebab-case` token, otherwise `other`.
#[must_use]
pub fn label(value: &str) -> &str {
    let ok = !value.is_empty()
        && value.len() <= 40
        && value
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_' || b == b'-');
    if ok {
        value
    } else if value.is_empty() {
        "none"
    } else {
        "other"
    }
}

/// Everything Sesame appends to `/metrics` besides Lifeguard: token-status counters and the default registry.
#[must_use]
pub fn prometheus_scrape_text() -> String {
    let mut text = crate::token_status::token_status_prometheus_scrape_text();
    let mut buf = Vec::new();
    if TextEncoder::new()
        .encode(&prometheus::gather(), &mut buf)
        .is_ok()
    {
        text.push('\n');
        text.push_str(&String::from_utf8_lossy(&buf));
    }
    text
}

#[cfg(test)]
mod tests {
    //! Label hygiene and the scrape text: free text never becomes a label, and a counter used once appears in
    //! the rendered text with its labels.
    use super::*;

    #[test]
    fn label_keeps_tokens_and_folds_free_text() {
        assert_eq!(label("wrong_password"), "wrong_password");
        assert_eq!(label("authz-core"), "authz-core");
        assert_eq!(label("Token family revoked: abc"), "other");
        assert_eq!(label("api_key_id=42"), "other");
        assert_eq!(label(""), "none");
        assert_eq!(label(&"a".repeat(41)), "other");
    }

    #[test]
    fn scrape_text_carries_registered_series() {
        MAIL_SENDS.with_label_values(&["otp", "sent"]).inc();
        DEPENDENCY_ERRORS
            .with_label_values(&["redis", "test"])
            .inc();
        let text = prometheus_scrape_text();
        assert!(text.contains("sesame_token_status_checks_total{result=\"active\"}"));
        assert!(text.contains("sesame_mail_send_total{kind=\"otp\",outcome=\"sent\"}"));
        assert!(text
            .contains("sesame_dependency_errors_total{component=\"test\",dependency=\"redis\"}"));
    }
}
