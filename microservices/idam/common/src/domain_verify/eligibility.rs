//! Which domains may be claimed at all.
//!
//! The Public Suffix List is compiled in through the `psl` crate, pinned to an exact version in
//! `Cargo.toml`; bumping it refreshes the list and is part of each release.

use super::{normalize_domain, Domain, DomainError};

/// Consumer mail providers: nobody can claim these, or their subdomains, for an organisation.
///
/// An entry ending in `.*` matches a registrable domain whose first label is the prefix under any public
/// suffix (`gmx.*` covers `gmx.de`, `gmx.net`, `gmx.co.uk`).
pub const DEFAULT_CONSUMER_PROVIDERS: &[&str] = &[
    "126.com",
    "163.com",
    "aim.com",
    "aol.*",
    "btinternet.com",
    "comcast.net",
    "fastmail.com",
    "fastmail.fm",
    "gmail.com",
    "gmx.*",
    "googlemail.com",
    "hey.com",
    "hotmail.*",
    "icloud.com",
    "inbox.com",
    "laposte.net",
    "live.*",
    "mac.com",
    "mail.com",
    "mail.ru",
    "me.com",
    "msn.com",
    "naver.com",
    "orange.fr",
    "outlook.*",
    "pm.me",
    "proton.me",
    "protonmail.ch",
    "protonmail.com",
    "qq.com",
    "rediffmail.com",
    "rocketmail.com",
    "sky.com",
    "t-online.de",
    "tuta.com",
    "tuta.io",
    "tutanota.com",
    "tutanota.de",
    "web.de",
    "yahoo.*",
    "yandex.*",
    "ymail.com",
    "zohomail.com",
];

#[derive(Debug, Clone, PartialEq, Eq)]
enum ProviderRule {
    /// The domain and its subdomains.
    Exact(Domain),
    /// Any registrable domain whose first label is this.
    AnySuffix(String),
}

/// Deployment-specific eligibility rules on top of the Public Suffix List.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EligibilityPolicy {
    providers: Vec<ProviderRule>,
    reserved: Vec<Domain>,
}

impl Default for EligibilityPolicy {
    fn default() -> Self {
        let mut policy = Self {
            providers: Vec::new(),
            reserved: Vec::new(),
        };
        for entry in DEFAULT_CONSUMER_PROVIDERS {
            // The built-in list is valid by construction (unit-tested).
            if let Some(rule) = parse_provider(entry) {
                policy.providers.push(rule);
            }
        }
        policy
    }
}

fn parse_provider(entry: &str) -> Option<ProviderRule> {
    if let Some(prefix) = entry.strip_suffix(".*") {
        let label = normalize_domain(prefix).ok()?;
        if label.labels().count() != 1 {
            return None;
        }
        return Some(ProviderRule::AnySuffix(label.as_str().to_string()));
    }
    normalize_domain(entry).ok().map(ProviderRule::Exact)
}

impl EligibilityPolicy {
    /// Add consumer providers (same syntax as [`DEFAULT_CONSUMER_PROVIDERS`]).
    ///
    /// # Errors
    ///
    /// [`DomainError::InvalidDomain`] for an entry that is not a domain or a `label.*` pattern.
    pub fn with_providers<I, S>(mut self, entries: I) -> Result<Self, DomainError>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        for entry in entries {
            let rule = parse_provider(entry.as_ref())
                .ok_or(DomainError::InvalidDomain("provider entry"))?;
            self.providers.push(rule);
        }
        Ok(self)
    }

    /// Add the deployment's own domains (its issuer and service hosts); they and their subdomains are
    /// never claimable.
    ///
    /// # Errors
    ///
    /// [`DomainError::InvalidDomain`] for an entry that does not normalise.
    pub fn with_reserved<I, S>(mut self, domains: I) -> Result<Self, DomainError>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        for domain in domains {
            self.reserved.push(normalize_domain(domain.as_ref())?);
        }
        Ok(self)
    }
}

/// Decide whether `domain` may be claimed.
///
/// Order of checks: public suffix (or above a registrable domain), unlisted top-level domain, consumer mail
/// provider, reserved domain.
///
/// # Errors
///
/// [`DomainError::PublicSuffix`], [`DomainError::UnknownSuffix`], [`DomainError::ConsumerProvider`] or
/// [`DomainError::ReservedDomain`].
pub fn check_eligible(domain: &Domain, policy: &EligibilityPolicy) -> Result<(), DomainError> {
    let bytes = domain.as_str().as_bytes();
    let suffix = psl::suffix(bytes).ok_or(DomainError::PublicSuffix)?;
    let registrable = psl::domain(bytes).ok_or(DomainError::PublicSuffix)?;
    if !suffix.is_known() {
        return Err(DomainError::UnknownSuffix);
    }
    let registrable = std::str::from_utf8(registrable.as_bytes())
        .map_err(|_| DomainError::InvalidDomain("not ASCII"))?;
    let registrable_first = registrable.split('.').next().unwrap_or_default();

    for rule in &policy.providers {
        let hit = match rule {
            ProviderRule::Exact(provider) => domain.is_same_or_under(provider),
            ProviderRule::AnySuffix(label) => registrable_first == label,
        };
        if hit {
            return Err(DomainError::ConsumerProvider);
        }
    }
    if policy.reserved.iter().any(|r| domain.is_same_or_under(r)) {
        return Err(DomainError::ReservedDomain);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check(input: &str, policy: &EligibilityPolicy) -> Result<(), &'static str> {
        let domain = normalize_domain(input).map_err(|e| e.code())?;
        check_eligible(&domain, policy).map_err(|e| e.code())
    }

    #[test]
    fn built_in_provider_list_parses_completely() {
        assert_eq!(
            EligibilityPolicy::default().providers.len(),
            DEFAULT_CONSUMER_PROVIDERS.len()
        );
    }

    #[test]
    fn public_suffixes_are_refused() {
        let p = EligibilityPolicy::default();
        for d in [
            "com",
            "co.uk",
            "github.io",
            "uk",
            "pages.dev",
            "s3.amazonaws.com",
        ] {
            assert_eq!(check(d, &p), Err("public_suffix"), "{d}");
        }
    }

    #[test]
    fn unlisted_tlds_are_refused() {
        let p = EligibilityPolicy::default();
        for d in [
            "bigbank.internal",
            "bigbank.local",
            "bigbank.example",
            "host.localdomain",
        ] {
            assert_eq!(check(d, &p), Err("unknown_suffix"), "{d}");
        }
        assert_eq!(check("localhost", &p), Err("public_suffix"));
    }

    #[test]
    fn consumer_providers_are_refused_with_subdomains_and_wildcards() {
        let p = EligibilityPolicy::default();
        for d in [
            "gmail.com",
            "GMail.com.",
            "outlook.com",
            "mail.outlook.com",
            "hotmail.co.uk",
            "gmx.de",
            "yahoo.co.jp",
            "proton.me",
        ] {
            assert_eq!(check(d, &p), Err("consumer_provider"), "{d}");
        }
    }

    #[test]
    fn organisations_are_accepted() {
        let p = EligibilityPolicy::default();
        for d in [
            "bigbank.com",
            "eu.bigbank.co.uk",
            "bigbank.co.uk",
            "gmailer.com",
            "notgmail.com",
            "user.github.io",
            "xn--bcher-kva.de",
        ] {
            assert_eq!(check(d, &p), Ok(()), "{d}");
        }
    }

    #[test]
    fn deployment_domains_are_reserved() {
        let p = EligibilityPolicy::default()
            .with_reserved(["sesame-idam.pricewhisperer.ai"])
            .expect("valid");
        assert_eq!(
            check("sesame-idam.pricewhisperer.ai", &p),
            Err("reserved_domain")
        );
        assert_eq!(
            check("x.sesame-idam.pricewhisperer.ai", &p),
            Err("reserved_domain")
        );
        assert_eq!(check("pricewhisperer.ai", &p), Ok(()));
    }

    #[test]
    fn extra_providers_extend_the_list() {
        let p = EligibilityPolicy::default()
            .with_providers(["freemail.example.com", "postbox.*"])
            .expect("valid");
        assert_eq!(check("freemail.example.com", &p), Err("consumer_provider"));
        assert_eq!(check("postbox.org", &p), Err("consumer_provider"));
        assert!(EligibilityPolicy::default()
            .with_providers(["a.b.*"])
            .is_err());
    }
}
