//! Security events on a user's sign-in methods: always audited, and announced to the account's email address, so an
//! attacker who adds or removes a factor, or signs in with a recovery code, cannot do it unseen (ADR-008 §6).
//!
//! Mail goes out on its own coroutine: a slow relay never delays the sign-in, and a failed send is logged and counted
//! (`sesame_mail_sends_total`) but never fails the operation that triggered it.

use sesame_common::audit::{AuditEventType, AuditLogEntry};

use crate::audit::EMITTER;

/// What happened.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    TotpAdded,
    PasskeyAdded,
    FactorRemoved,
    RecoveryCodesGenerated,
    RecoveryCodeUsed,
}

impl Event {
    #[must_use]
    pub const fn reason(self) -> &'static str {
        match self {
            Self::TotpAdded => "mfa_totp_enrolled",
            Self::PasskeyAdded => "mfa_passkey_registered",
            Self::FactorRemoved => "mfa_factor_removed",
            Self::RecoveryCodesGenerated => "mfa_recovery_codes_generated",
            Self::RecoveryCodeUsed => "mfa_recovery_code_used",
        }
    }

    fn subject(self) -> &'static str {
        match self {
            Self::TotpAdded => "Security alert: an authenticator app was added to your account",
            Self::PasskeyAdded => "Security alert: a passkey was added to your account",
            Self::FactorRemoved => "Security alert: a sign-in method was removed from your account",
            Self::RecoveryCodesGenerated => "Security alert: new recovery codes were generated",
            Self::RecoveryCodeUsed => "Security alert: a recovery code was used to sign in",
        }
    }

    fn body(self, detail: &str) -> String {
        let what = match self {
            Self::TotpAdded => format!("An authenticator app ({detail}) was added as a sign-in method."),
            Self::PasskeyAdded => format!("A passkey ({detail}) was added as a sign-in method."),
            Self::FactorRemoved => format!("A sign-in method ({detail}) was removed."),
            Self::RecoveryCodesGenerated => {
                "A new set of recovery codes was generated. Your previous codes no longer work.".to_string()
            }
            Self::RecoveryCodeUsed => format!(
                "Someone signed in to your account with one of your recovery codes ({detail} left). If you lost your \
                 authenticator, add a new one and generate new recovery codes."
            ),
        };
        format!(
            "{what}\n\nWhen: {} UTC\n\nIf this was you, there is nothing to do. If it was not, reset your password \
             now and contact support: someone else may be able to sign in as you.\n",
            chrono::Utc::now().format("%Y-%m-%d %H:%M")
        )
    }
}

/// Audit the event and mail the account holder.
pub fn announce(tenant_id: &str, user_id: &uuid::Uuid, email: &str, event: Event, detail: &str) {
    audit(tenant_id, user_id, event);
    if email.trim().is_empty() {
        return;
    }
    let (to, subject, body) = (email.to_string(), event.subject(), event.body(detail));
    may::go!(move || {
        if let Err(error) = crate::services::email::send_email(&to, subject, &body) {
            tracing::warn!(error = %format!("{error:#}"), "mfa: security notification not sent");
        }
    });
}

fn audit(tenant_id: &str, user_id: &uuid::Uuid, event: Event) {
    match AuditLogEntry::new(
        AuditEventType::AuthenticatorChanged,
        "identity-login-service",
    )
    .tenant_id(tenant_id.to_string())
    .user_id(user_id.to_string())
    .decision_source("mfa")
    .result("allowed")
    .reason(event.reason().to_string())
    .build()
    {
        Ok(entry) => EMITTER.emit(entry),
        Err(e) => tracing::warn!(error = %e, "mfa: audit entry build failed"),
    }
}

/// Audit a refused second-factor attempt (wrong code, unknown passkey, burned context).
pub fn audit_denied(tenant_id: &str, user_id: Option<&uuid::Uuid>, reason: &str) {
    let mut builder =
        AuditLogEntry::new(AuditEventType::ValidationFailed, "identity-login-service")
            .tenant_id(tenant_id.to_string())
            .decision_source("mfa")
            .result("denied")
            .reason(reason.to_string());
    if let Some(u) = user_id {
        builder = builder.user_id(u.to_string());
    }
    match builder.build() {
        Ok(entry) => EMITTER.emit(entry),
        Err(e) => tracing::warn!(error = %e, "mfa: audit entry build failed"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_notification_tells_the_user_what_to_do_if_it_was_not_them() {
        for e in [
            Event::TotpAdded,
            Event::PasskeyAdded,
            Event::FactorRemoved,
            Event::RecoveryCodesGenerated,
            Event::RecoveryCodeUsed,
        ] {
            assert!(e.subject().starts_with("Security alert"));
            assert!(e.body("x").contains("If it was not"));
        }
    }
}
