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
//!   DB. Both tenant tiers arrive in Phase 2; until then a tenant-billed
//!   purpose resolves to `Fallback` and the caller uses email or refuses —
//!   NEVER the platform credential (ADR-009 §2.5, no silent subsidy).

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
        OwnerKind::Tenant => {
            // Phase 2 wires Connect / envelope custody from
            // `tenant_sms_config`. Until then: no tenant sender exists, so we
            // refuse rather than silently charging the platform.
            let _ = environment;
            Err(Unresolved::NoTenantSender {
                tenant: tenant.to_string(),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

    /// A tenant-billed purpose with no tenant sender must NOT fall back to the
    /// platform credential (ADR-009 §2.5).
    #[test]
    fn missing_tenant_sender_refuses_rather_than_subsidising() {
        std::env::set_var("SMS_ALLOWED_PURPOSES", "registration,password_reset");
        let err = resolve_sms_sender("hauliage", "dev", SmsPurpose::Registration)
            .expect_err("must not resolve without a tenant sender");
        assert!(matches!(err, Unresolved::NoTenantSender { .. }));
    }

    #[test]
    fn platform_sender_has_its_own_spend_scope() {
        std::env::set_var("SMS_ALLOWED_PURPOSES", "tenant_registration");
        let sender = resolve_sms_sender("any", "dev", SmsPurpose::TenantRegistration)
            .expect("platform purposes resolve");
        assert_eq!(sender.owner, BillingOwner::Platform);
        assert_eq!(sender.credential, Credential::PlatformEnv);
        assert_eq!(sender.spend_scope, "platform");
    }

    #[test]
    fn disallowed_purpose_is_refused() {
        std::env::set_var("SMS_ALLOWED_PURPOSES", "registration");
        assert_eq!(
            resolve_sms_sender("t", "dev", SmsPurpose::Login).unwrap_err(),
            Unresolved::PurposeNotAllowed
        );
    }
}
