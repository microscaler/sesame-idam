//! Tenant assurance levels and the KYB/KYC provider seam.
//!
//! # Why a ladder rather than a gate
//!
//! "Is this a real company, and is this person allowed to act for it?" has
//! answers of very different cost:
//!
//! | Level | Proven by | Marginal cost |
//! | --- | --- | --- |
//! | [`EmailVerified`] | possession of an inbox | zero |
//! | [`DomainVerified`] | a DNS TXT record in the company's zone (ADR-007) | zero |
//! | [`BusinessVerified`] | a KYB vendor checking registration documents | per-check, plus a subscription |
//!
//! Charging the top rung at the front door would mean paying for every tyre
//! kicker before knowing whether they are a customer. So capability gates on
//! the level a tenant *has*, and each risky action names the level it needs.
//! Most tenants never need the paid rung.
//!
//! For B2B SaaS, [`DomainVerified`] carries most of the weight: controlling
//! `acme.com`'s DNS is a strong claim to acting for Acme, it costs nothing, and
//! it needs no vendor relationship.
//!
//! # Why the provider seam exists while disabled
//!
//! [`BusinessVerified`] needs a vendor (Persona, Stripe Identity, Sumsub —
//! Twilio does not sell this; they use Persona themselves). At roughly
//! USD 250/month that is not a pre-revenue expense, so the default provider is
//! [`KycProvider::Disabled`].
//!
//! The seam is built anyway because retrofitting an assurance concept into
//! authorisation checks that never had one is how you get a half-migrated
//! model with two notions of "verified". Deciding the shape now, while there
//! are three call sites, is cheap; deciding it later is not.
//!
//! Enabling a provider later must not retroactively break anyone: existing
//! tenants keep the level they earned, and the new provider only gates
//! capabilities that *require* the top rung. Turning the flag on adds a gate;
//! it does not revoke a grant.

use std::fmt;

/// How much is known about who a tenant actually is. Ordered: each level
/// implies the ones below it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AssuranceLevel {
    /// Nothing proven. A freshly signed-up tenant before it confirms anything.
    Unverified,
    /// Someone read a message we sent to the address they claimed.
    EmailVerified,
    /// A DNS TXT record was published in the claimed domain's zone (ADR-007).
    DomainVerified,
    /// A KYB provider checked business registration documents.
    BusinessVerified,
}

impl AssuranceLevel {
    /// Parse a stored level. Unknown values read as [`Unverified`] rather than
    /// erroring: an unrecognised level must never be treated as *more* trusted
    /// than it is, and a typo in a migration should fail closed.
    ///
    /// [`Unverified`]: AssuranceLevel::Unverified
    #[must_use]
    pub fn parse(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "email_verified" => Self::EmailVerified,
            "domain_verified" => Self::DomainVerified,
            "business_verified" => Self::BusinessVerified,
            _ => Self::Unverified,
        }
    }

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Unverified => "unverified",
            Self::EmailVerified => "email_verified",
            Self::DomainVerified => "domain_verified",
            Self::BusinessVerified => "business_verified",
        }
    }
}

impl fmt::Display for AssuranceLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Things a tenant might do that warrant knowing who they are.
///
/// Naming the capability rather than the level at each call site means the
/// policy lives in one table below, and raising a requirement is one edit
/// instead of a hunt through controllers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Capability {
    /// Use the platform at all — sign in, invite colleagues, read the console.
    UseConsole,
    /// Configure an SSO provider for the tenant's own users.
    ConfigureSso,
    /// Send SMS to end users. Costs real money and reaches strangers' phones,
    /// which is why it sits above the free rungs.
    SendSms,
    /// Have Sesame hold the tenant's own provider credentials (ADR-009
    /// envelope custody). We take on custody of someone's secret, so we should
    /// know whose.
    StoreProviderCredentials,
    /// Move to production, where real end users are affected.
    ProductionAccess,
}

impl Capability {
    /// The minimum assurance a capability requires.
    ///
    /// `SendSms` and `ProductionAccess` sit at [`DomainVerified`] rather than
    /// [`BusinessVerified`] deliberately: with no KYB provider configured,
    /// requiring the top rung would make them unreachable for everyone, and a
    /// capability nobody can reach is an outage dressed as a policy. When a
    /// provider is enabled these are the first candidates to be raised.
    ///
    /// [`DomainVerified`]: AssuranceLevel::DomainVerified
    /// [`BusinessVerified`]: AssuranceLevel::BusinessVerified
    #[must_use]
    pub const fn required_assurance(self) -> AssuranceLevel {
        match self {
            Self::UseConsole => AssuranceLevel::EmailVerified,
            Self::ConfigureSso => AssuranceLevel::DomainVerified,
            Self::SendSms => AssuranceLevel::DomainVerified,
            Self::StoreProviderCredentials => AssuranceLevel::DomainVerified,
            Self::ProductionAccess => AssuranceLevel::DomainVerified,
        }
    }

    /// Whether a tenant at `have` may exercise this capability.
    #[must_use]
    pub fn permitted_at(self, have: AssuranceLevel) -> bool {
        have >= self.required_assurance()
    }
}

/// Which KYB provider backs [`AssuranceLevel::BusinessVerified`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KycProvider {
    /// No provider. `BusinessVerified` is unreachable through self-service;
    /// the ladder tops out at `DomainVerified`. This is the default, because a
    /// KYB subscription is not a pre-revenue expense.
    Disabled,
    /// A platform operator records the outcome of an out-of-band check.
    /// Not automated and not cheap in human time, but it makes the top rung
    /// reachable for a handful of high-value tenants without a subscription.
    Manual,
    /// A hosted KYB vendor (Persona and similar). Not implemented — the
    /// variant exists so the seam is real rather than hypothetical, and so
    /// enabling it is a config change plus one implementation, not a redesign.
    Hosted,
}

/// Env var selecting the provider. Absent or unrecognised means
/// [`KycProvider::Disabled`] — the safe reading, since the alternative is
/// silently believing a provider is checking things when none is.
pub const KYC_PROVIDER_ENV: &str = "SESAME_KYC_PROVIDER";

impl KycProvider {
    #[must_use]
    pub fn from_env() -> Self {
        match std::env::var(KYC_PROVIDER_ENV)
            .unwrap_or_default()
            .trim()
            .to_ascii_lowercase()
            .as_str()
        {
            "manual" => Self::Manual,
            "hosted" | "persona" => Self::Hosted,
            _ => Self::Disabled,
        }
    }

    /// The highest level a tenant can reach with this provider configured.
    #[must_use]
    pub const fn ceiling(self) -> AssuranceLevel {
        match self {
            // Domain verification needs no vendor, so the ladder still reaches
            // it with KYB switched off entirely.
            Self::Disabled => AssuranceLevel::DomainVerified,
            Self::Manual | Self::Hosted => AssuranceLevel::BusinessVerified,
        }
    }

    /// Whether business verification can currently be obtained at all.
    #[must_use]
    pub fn business_verification_available(self) -> bool {
        self.ceiling() >= AssuranceLevel::BusinessVerified
    }
}

/// Why a capability was refused — enough for the console to say something
/// useful ("verify your domain to enable SMS") rather than a bare 403.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssuranceShortfall {
    pub capability_requires: AssuranceLevel,
    pub tenant_has: AssuranceLevel,
    /// True when the requirement cannot currently be met by any self-service
    /// route, because it needs a provider that is not configured. That is an
    /// operator problem, not a tenant one, and should be reported differently.
    pub unreachable: bool,
}

/// Check a capability against a tenant's assurance.
///
/// # Errors
///
/// Returns the shortfall when the tenant is below the required level.
pub fn require_capability(
    capability: Capability,
    tenant_level: AssuranceLevel,
    provider: KycProvider,
) -> Result<(), AssuranceShortfall> {
    let required = capability.required_assurance();
    if tenant_level >= required {
        return Ok(());
    }
    Err(AssuranceShortfall {
        capability_requires: required,
        tenant_has: tenant_level,
        unreachable: required > provider.ceiling(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn levels_are_ordered_so_higher_implies_lower() {
        assert!(AssuranceLevel::DomainVerified > AssuranceLevel::EmailVerified);
        assert!(AssuranceLevel::BusinessVerified > AssuranceLevel::DomainVerified);
        assert!(AssuranceLevel::EmailVerified > AssuranceLevel::Unverified);
    }

    /// An unreadable level must never be mistaken for a verified one.
    #[test]
    fn unknown_stored_level_reads_as_unverified() {
        assert_eq!(AssuranceLevel::parse("wat"), AssuranceLevel::Unverified);
        assert_eq!(AssuranceLevel::parse(""), AssuranceLevel::Unverified);
        assert_eq!(
            AssuranceLevel::parse("DOMAIN_VERIFIED"),
            AssuranceLevel::DomainVerified
        );
    }

    /// The point of the whole module: with KYB off, a tenant can still earn
    /// every capability the product needs. A disabled provider must not be an
    /// outage.
    #[test]
    fn every_capability_is_reachable_with_kyb_disabled() {
        let ceiling = KycProvider::Disabled.ceiling();
        for capability in [
            Capability::UseConsole,
            Capability::ConfigureSso,
            Capability::SendSms,
            Capability::StoreProviderCredentials,
            Capability::ProductionAccess,
        ] {
            assert!(
                capability.required_assurance() <= ceiling,
                "{capability:?} is unreachable without a KYB provider"
            );
        }
    }

    #[test]
    fn email_alone_does_not_buy_sms() {
        let shortfall = require_capability(
            Capability::SendSms,
            AssuranceLevel::EmailVerified,
            KycProvider::Disabled,
        )
        .expect_err("email is not enough to send SMS");
        assert_eq!(
            shortfall.capability_requires,
            AssuranceLevel::DomainVerified
        );
        assert!(
            !shortfall.unreachable,
            "domain verification is self-service, so this is on the tenant to fix"
        );
    }

    #[test]
    fn domain_verification_unlocks_the_product() {
        for capability in [
            Capability::UseConsole,
            Capability::ConfigureSso,
            Capability::SendSms,
            Capability::StoreProviderCredentials,
        ] {
            assert!(
                require_capability(
                    capability,
                    AssuranceLevel::DomainVerified,
                    KycProvider::Disabled
                )
                .is_ok(),
                "{capability:?} should be available to a domain-verified tenant"
            );
        }
    }

    /// If a requirement is ever raised above what the configured provider can
    /// grant, the refusal has to say so — that is an operator misconfiguration,
    /// not something the tenant can resolve by trying harder.
    #[test]
    fn unreachable_requirement_is_reported_as_such() {
        let shortfall = require_capability(
            Capability::SendSms,
            AssuranceLevel::EmailVerified,
            // Pretend SendSms had been raised to BusinessVerified while the
            // provider stayed off: the ceiling is what makes it unreachable.
            KycProvider::Disabled,
        )
        .expect_err("below requirement");
        assert!(
            !shortfall.unreachable,
            "DomainVerified is within the ceiling"
        );

        assert!(
            AssuranceLevel::BusinessVerified > KycProvider::Disabled.ceiling(),
            "business verification is out of reach while KYB is disabled"
        );
        assert!(!KycProvider::Disabled.business_verification_available());
        assert!(KycProvider::Manual.business_verification_available());
    }

    #[test]
    fn provider_defaults_to_disabled_on_absent_or_junk_config() {
        let _guard = ENV_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let restore = std::env::var(KYC_PROVIDER_ENV).ok();

        std::env::remove_var(KYC_PROVIDER_ENV);
        assert_eq!(KycProvider::from_env(), KycProvider::Disabled);

        std::env::set_var(KYC_PROVIDER_ENV, "definitely-not-a-provider");
        assert_eq!(
            KycProvider::from_env(),
            KycProvider::Disabled,
            "a typo must not be read as 'a provider is checking things'"
        );

        std::env::set_var(KYC_PROVIDER_ENV, "manual");
        assert_eq!(KycProvider::from_env(), KycProvider::Manual);
        std::env::set_var(KYC_PROVIDER_ENV, "persona");
        assert_eq!(KycProvider::from_env(), KycProvider::Hosted);

        match restore {
            Some(v) => std::env::set_var(KYC_PROVIDER_ENV, v),
            None => std::env::remove_var(KYC_PROVIDER_ENV),
        }
    }
}
