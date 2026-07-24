//! SMS sender resolution — who sends, who pays (ADR-009 §2.1–2.2).
//!
//! # The security-critical invariant
//!
//! `purpose → billing owner` is a **server-side constant map**. It is never
//! derived from request input. This is the confused-deputy guard: without it,
//! a tenant's end-user flow could be steered into billing the PLATFORM's
//! Twilio account (or another tenant's).
//!
//! # The rule
//!
//! The account that sends and pays is whoever owns the *relationship* the
//! message serves — decided by which console/app the human is authenticating
//! into:
//!
//! - Platform-level identity ops (tenant onboarding, environment
//!   provisioning, tenant-OWNER recovery, operator MFA) → **platform**.
//! - A tenant's own end-users inside the tenant's app (registration, reset,
//!   phone re-verification, opt-in login MFA) → **tenant**.
//!
//! Tenant-owner recovery is platform-billed on purpose: it restores access to
//! the tenant *on Sesame*, not to the tenant's application.
//!
//! # Custody tiers (ADR-009 §2.3)
//!
//! - Platform: one credential set from the secret backend (SOPS → Secret →
//!   env). Implemented here.
//! - Tenant, PREFERRED: Twilio Connect — Twilio bills the tenant directly and
//!   Sesame holds only a revocable connected AccountSid.
//! - Tenant, FALLBACK (dogfood only): envelope-encrypted credentials in the
//!   DB (`tenant_sms_config`), unsealed in-process at send time.
//!
//! Only an ACTIVE tenant config resolves. Missing, `pending_validation` or
//! `revoked` all yield `NoTenantSender`, and the caller then uses email or
//! refuses — NEVER the platform credential (ADR-009 §2.5, no silent subsidy).

use crate::services::sms::SmsPurpose;

/// Who pays for (and sends) a message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BillingOwner {
    /// The Sesame platform's own account.
    Platform,
    /// A tenant's account (Connect or envelope custody).
    Tenant(String),
}

/// Resolved sender: the credential to use plus the ceilings that bound it.
#[derive(Debug, Clone)]
pub struct SmsSender {
    pub owner: BillingOwner,
    pub credential: Credential,
    /// Daily spend ceiling (cents) for this owner.
    pub daily_ceiling_cents: u64,
    /// Key used for spend accounting — distinct per owner so budgets and
    /// blast radius never bleed between tenants or into the platform.
    pub spend_scope: String,
}

/// How we authenticate to the provider for this send.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Credential {
    /// Platform account from env (SOPS-delivered Secret).
    PlatformEnv,
    /// Twilio Connect: act on the tenant's connected account (Phase 2).
    TenantConnect { connected_account_sid: String },
    /// Envelope-decrypted tenant credentials (Phase 2, dogfood only).
    TenantEnvelope { account_sid: String, auth_token: String },
}

/// Why a send could not be attributed to a payer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Unresolved {
    /// Tenant-billed purpose, but the tenant has no usable sender configured.
    /// Caller MUST fall back to email or refuse — never bill the platform.
    NoTenantSender { tenant: String },
    /// Purpose is not permitted by the cost policy at all.
    PurposeNotAllowed,
}

/// **The confused-deputy guard.** Server-side, constant, request-independent.
#[must_use]
pub const fn billing_owner_for(purpose: SmsPurpose) -> OwnerKind {
    match purpose {
        // Platform relationship: onboarding a tenant, provisioning an
        // environment, recovering access to the Sesame console itself.
        SmsPurpose::TenantRegistration
        | SmsPurpose::EnvironmentRegistration
        | SmsPurpose::TenantOwnerRecovery
        | SmsPurpose::PlatformOperator => OwnerKind::Platform,
        // Tenant relationship: the tenant's own end-users.
        SmsPurpose::Registration
        | SmsPurpose::PasswordReset
        | SmsPurpose::PhoneReverification
        | SmsPurpose::Login
        | SmsPurpose::AccountRecovery => OwnerKind::Tenant,
    }
}

/// Coarse owner classification (the constant half of resolution).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OwnerKind {
    Platform,
    Tenant,
}

fn env_u64(name: &str, default: u64) -> u64 {
    std::env::var(name)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

/// Resolve the sender for `(tenant, environment, purpose)`.
///
/// # Errors
///
/// Returns [`Unresolved`] when the cost policy forbids the purpose, or when a
/// tenant-billed purpose has no tenant sender configured.
pub fn resolve_sms_sender(
    tenant: &str,
    environment: &str,
    purpose: SmsPurpose,
) -> Result<SmsSender, Unresolved> {
    if !crate::services::sms::purpose_allowed(purpose) {
        return Err(Unresolved::PurposeNotAllowed);
    }

    match billing_owner_for(purpose) {
        OwnerKind::Platform => Ok(SmsSender {
            owner: BillingOwner::Platform,
            credential: Credential::PlatformEnv,
            daily_ceiling_cents: env_u64("SMS_PLATFORM_DAILY_CEILING_CENTS", 1000),
            // Platform budget is global, not per tenant.
            spend_scope: "platform".to_string(),
        }),
        OwnerKind::Tenant => resolve_tenant_sender(tenant, environment),
    }
}

/// Look up the tenant's own sender (ADR-009 Phase 2).
///
/// Only an `active` config resolves: `pending_validation` (credentials not
/// yet proven) and `revoked` both fall through to `NoTenantSender`, so the
/// caller uses email or refuses — never the platform's account.
fn resolve_tenant_sender(tenant: &str, environment: &str) -> Result<SmsSender, Unresolved> {
    use crate::models::tenant_sms_config::{CUSTODY_CONNECT, CUSTODY_ENVELOPE, STATUS_ACTIVE};
    use crate::services::tenant_sms_service::TenantSmsService;

    let none = || Unresolved::NoTenantSender {
        tenant: tenant.to_string(),
    };

    let exec = sesame_idam_database::db();
    let config = TenantSmsService::find(tenant, environment, exec)
        .map_err(|e| {
            tracing::warn!(error = %e, tenant, environment, "tenant sms config lookup failed");
            none()
        })?
        .ok_or_else(none)?;

    if config.status != STATUS_ACTIVE {
        tracing::info!(
            tenant, environment, status = %config.status,
            "tenant sms config is not active — falling back"
        );
        return Err(none());
    }

    let credential = match config.custody_mode.as_str() {
        CUSTODY_CONNECT => Credential::TenantConnect {
            connected_account_sid: config.connected_account_sid.clone().ok_or_else(none)?,
        },
        CUSTODY_ENVELOPE => Credential::TenantEnvelope {
            account_sid: config.account_sid.clone().ok_or_else(none)?,
            // Unsealed only here, only for an active config.
            auth_token: TenantSmsService::resolve_credential(&config).ok_or_else(none)?,
        },
        other => {
            tracing::warn!(tenant, custody = other, "unknown custody mode");
            return Err(none());
        }
    };

    Ok(SmsSender {
        owner: BillingOwner::Tenant(tenant.to_string()),
        credential,
        daily_ceiling_cents: u64::try_from(config.daily_spend_ceiling_cents).unwrap_or(0),
        // Per-tenant scope: this tenant's spend can never touch another's or
        // the platform's budget.
        spend_scope: format!("tenant:{tenant}"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `SMS_ALLOWED_PURPOSES` is process-global; the parallel runner would
    /// otherwise let one test's policy leak into another's assertion.
    static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    struct PolicyGuard(Option<String>);

    impl PolicyGuard {
        fn set(value: &str) -> Self {
            let prior = std::env::var("SMS_ALLOWED_PURPOSES").ok();
            std::env::set_var("SMS_ALLOWED_PURPOSES", value);
            Self(prior)
        }
    }

    impl Drop for PolicyGuard {
        fn drop(&mut self) {
            match self.0.take() {
                Some(v) => std::env::set_var("SMS_ALLOWED_PURPOSES", v),
                None => std::env::remove_var("SMS_ALLOWED_PURPOSES"),
            }
        }
    }

    /// The invariant that matters: end-user purposes NEVER resolve to the
    /// platform's credential.
    #[test]
    fn tenant_purposes_never_bill_the_platform() {
        for purpose in [
            SmsPurpose::Registration,
            SmsPurpose::PasswordReset,
            SmsPurpose::PhoneReverification,
            SmsPurpose::Login,
            SmsPurpose::AccountRecovery,
        ] {
            assert_eq!(
                billing_owner_for(purpose),
                OwnerKind::Tenant,
                "{purpose:?} must be tenant-billed"
            );
        }
    }

    #[test]
    fn platform_purposes_bill_the_platform() {
        for purpose in [
            SmsPurpose::TenantRegistration,
            SmsPurpose::EnvironmentRegistration,
            SmsPurpose::TenantOwnerRecovery,
            SmsPurpose::PlatformOperator,
        ] {
            assert_eq!(billing_owner_for(purpose), OwnerKind::Platform);
        }
    }

    /// The two owner sets are disjoint and exhaustive — a purpose added later
    /// cannot quietly land in neither (the match is non-exhaustive-proof) nor
    /// in both.
    #[test]
    fn no_purpose_is_both_platform_and_tenant() {
        let platform = [
            SmsPurpose::TenantRegistration,
            SmsPurpose::EnvironmentRegistration,
            SmsPurpose::TenantOwnerRecovery,
            SmsPurpose::PlatformOperator,
        ];
        for p in platform {
            assert_ne!(billing_owner_for(p), OwnerKind::Tenant);
        }
    }

    /// A tenant-billed purpose must resolve against the TENANT, never the
    /// platform credential (ADR-009 §2.5). The DB-backed half of
    /// `resolve_tenant_sender` is covered by the BDD suite.
    #[test]
    fn tenant_purposes_resolve_to_tenant_kind() {
        let _guard = ENV_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let _policy = PolicyGuard::set("registration,password_reset");
        assert_eq!(
            billing_owner_for(SmsPurpose::Registration),
            OwnerKind::Tenant,
            "registration must never resolve to the platform credential"
        );
    }

    #[test]
    fn platform_sender_has_its_own_spend_scope() {
        let _guard = ENV_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let _policy = PolicyGuard::set("tenant_registration");
        let sender = resolve_sms_sender("any", "dev", SmsPurpose::TenantRegistration)
            .expect("platform purposes resolve");
        assert_eq!(sender.owner, BillingOwner::Platform);
        assert_eq!(sender.credential, Credential::PlatformEnv);
        assert_eq!(
            sender.spend_scope, "platform",
            "platform spend must not be attributed to a tenant budget"
        );
    }

    #[test]
    fn disallowed_purpose_is_refused() {
        let _guard = ENV_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let _policy = PolicyGuard::set("registration");
        assert_eq!(
            resolve_sms_sender("t", "dev", SmsPurpose::Login).unwrap_err(),
            Unresolved::PurposeNotAllowed
        );
    }
}
