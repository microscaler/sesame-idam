//! What a sign-in ceremony proved, as token claims (ADR-008 §3): `amr` (RFC 8176), `acr` and `auth_time` (OIDC Core).
//!
//! Sesame asserts what happened; resources decide whether it is enough. The one rule this module exists to keep is
//! that the claims are never inflated:
//!
//! * `mfa` is added only when two **independent** factor categories were used. An emailed code or link is not a
//!   factor independent of the password: whoever holds the mailbox can reset the password (ADR-008 "Factor
//!   honesty"). So it is counted in the password's category: password + email code is single-factor, and an email
//!   code is never what makes a ceremony `mfa`. (Email code + TOTP is two factors because of the TOTP, exactly as
//!   password + TOTP is.)
//! * A passkey verified with user verification is multi-factor in one gesture (the authenticator, unlocked by a
//!   biometric or PIN) and the only phishing-resistant method: it alone earns [`ACR_AAL2_PHISHING_RESISTANT`].
//! * A recovery code is a real possession factor (a printed look-up secret, NIST SP 800-63B §5.1.2), so password +
//!   recovery code is `aal2`, but never phishing-resistant: a member of an organisation that requires passkeys who
//!   recovers with a code is not issued a token until a new passkey is enrolled (ADR-008 §6: recovery never silently
//!   downgrades assurance; the policy check at issuance sees `aal2` < required).
//!
//! `amr` values: RFC 8176 where one exists (`pwd`, `otp` for TOTP, `sms`, `hwk`, `swk`, `user`, `mfa`), and three
//! Sesame values where RFC 8176 has none, so a resource can never mistake them for something stronger: `email`
//! (emailed code or magic link), `rc` (recovery code), `fed` (a social / federated provider).

use serde::{Deserialize, Serialize};

/// Single factor (or an email code alone).
pub const ACR_AAL1: &str = "urn:sesame:acr:aal1";
/// Two independent factors, at least one phishable (password + TOTP, password + recovery code).
pub const ACR_AAL2: &str = "urn:sesame:acr:aal2";
/// A passkey with user verification: multi-factor and bound to the origin.
pub const ACR_AAL2_PHISHING_RESISTANT: &str = "urn:sesame:acr:aal2-phishing-resistant";

/// One method a ceremony used.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "method", rename_all = "snake_case")]
pub enum Method {
    /// The account password.
    Password,
    /// A TOTP code (RFC 6238) from an enrolled authenticator app.
    Totp,
    /// A one-time recovery code.
    RecoveryCode,
    /// A `WebAuthn` passkey. `user_verified`: the authenticator verified the user (biometric / PIN);
    /// `synced`: the credential is backup-eligible (iCloud Keychain, Google Password Manager…), else device-bound.
    Passkey { user_verified: bool, synced: bool },
    /// A code or link sent to the account's email address.
    Email,
    /// A code sent by SMS (NIST "restricted").
    Sms,
    /// A social / federated identity provider.
    Federated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Category {
    Knowledge,
    Possession,
    Inherence,
    External,
}

impl Method {
    /// The RFC 8176 (or Sesame) `amr` values this method contributes.
    fn amr(self) -> &'static [&'static str] {
        match self {
            Self::Password => &["pwd"],
            Self::Totp => &["otp"],
            Self::RecoveryCode => &["rc"],
            Self::Passkey {
                user_verified: true,
                synced: true,
            } => &["swk", "user"],
            Self::Passkey {
                user_verified: true,
                synced: false,
            } => &["hwk", "user"],
            Self::Passkey {
                user_verified: false,
                synced: true,
            } => &["swk"],
            Self::Passkey {
                user_verified: false,
                synced: false,
            } => &["hwk"],
            Self::Email => &["email"],
            Self::Sms => &["sms"],
            Self::Federated => &["fed"],
        }
    }

    /// The independent factor categories it proves. Email is the password's category (see the module docs).
    fn categories(self) -> &'static [Category] {
        match self {
            Self::Password | Self::Email => &[Category::Knowledge],
            Self::Passkey {
                user_verified: true,
                ..
            } => &[Category::Possession, Category::Inherence],
            Self::Totp | Self::RecoveryCode | Self::Sms | Self::Passkey { .. } => {
                &[Category::Possession]
            }
            Self::Federated => &[Category::External],
        }
    }
}

/// Assurance ranking, for comparing what a ceremony achieved with what a policy requires.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Assurance {
    Aal1,
    Aal2,
    Aal2PhishingResistant,
}

impl Assurance {
    #[must_use]
    pub const fn acr(self) -> &'static str {
        match self {
            Self::Aal1 => ACR_AAL1,
            Self::Aal2 => ACR_AAL2,
            Self::Aal2PhishingResistant => ACR_AAL2_PHISHING_RESISTANT,
        }
    }

    /// Parse an `acr` value. Unknown values read as [`Assurance::Aal1`]: an unrecognised class is never treated as
    /// stronger than it is.
    #[must_use]
    pub fn from_acr(acr: &str) -> Self {
        match acr {
            ACR_AAL2_PHISHING_RESISTANT => Self::Aal2PhishingResistant,
            ACR_AAL2 => Self::Aal2,
            _ => Self::Aal1,
        }
    }
}

/// The authentication context carried by access, refresh and ID tokens.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Authn {
    /// RFC 8176 method references, sorted and de-duplicated.
    pub amr: Vec<String>,
    /// The assurance class the ceremony satisfied.
    pub acr: String,
    /// When the user actually authenticated (unix seconds). Kept, never refreshed, across token refreshes.
    pub auth_time: i64,
}

impl Authn {
    /// The truthful claims for a ceremony that used `methods` at `auth_time`.
    #[must_use]
    pub fn from_methods(methods: &[Method], auth_time: i64) -> Self {
        let mut amr: Vec<String> = methods
            .iter()
            .flat_map(|m| m.amr().iter().map(|s| (*s).to_string()))
            .collect();
        let mut categories: Vec<Category> = methods
            .iter()
            .flat_map(|m| m.categories().iter().copied())
            .collect();
        categories.sort();
        categories.dedup();
        let multi_factor = categories.len() >= 2;
        let phishing_resistant = methods.iter().any(|m| {
            matches!(
                m,
                Method::Passkey {
                    user_verified: true,
                    ..
                }
            )
        });
        if multi_factor {
            amr.push("mfa".to_string());
        }
        amr.sort();
        amr.dedup();
        let assurance = if phishing_resistant {
            Assurance::Aal2PhishingResistant
        } else if multi_factor {
            Assurance::Aal2
        } else {
            Assurance::Aal1
        };
        Self {
            amr,
            acr: assurance.acr().to_string(),
            auth_time,
        }
    }

    /// [`Authn::from_methods`] now.
    #[must_use]
    pub fn now(methods: &[Method]) -> Self {
        Self::from_methods(methods, chrono::Utc::now().timestamp())
    }

    #[must_use]
    pub fn assurance(&self) -> Assurance {
        Assurance::from_acr(&self.acr)
    }

    /// Whether `amr` contains `value`.
    #[must_use]
    pub fn has(&self, value: &str) -> bool {
        self.amr.iter().any(|v| v == value)
    }

    /// Read the context from a token's claims (`amr`, `acr`, `auth_time`). `None` when the token predates these
    /// claims (or lacks any of them): callers must then not claim more than they can prove.
    #[must_use]
    pub fn from_claims(claims: &serde_json::Value) -> Option<Self> {
        let amr = claims
            .get("amr")?
            .as_array()?
            .iter()
            .filter_map(|v| v.as_str().map(str::to_string))
            .collect::<Vec<_>>();
        let acr = claims.get("acr")?.as_str()?.to_string();
        let auth_time = claims.get("auth_time")?.as_i64()?;
        Some(Self {
            amr,
            acr,
            auth_time,
        })
    }

    /// Write the claims into a JSON object (an ID token or refresh token payload).
    pub fn write_into(&self, payload: &mut serde_json::Value) {
        if let Some(map) = payload.as_object_mut() {
            map.insert("amr".into(), serde_json::json!(self.amr));
            map.insert("acr".into(), serde_json::json!(self.acr));
            map.insert("auth_time".into(), serde_json::json!(self.auth_time));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const T: i64 = 1_800_000_000;

    fn passkey(synced: bool) -> Method {
        Method::Passkey {
            user_verified: true,
            synced,
        }
    }

    #[test]
    fn password_alone_is_aal1() {
        let a = Authn::from_methods(&[Method::Password], T);
        assert_eq!(a.amr, ["pwd"]);
        assert_eq!(a.acr, ACR_AAL1);
        assert_eq!(a.auth_time, T);
    }

    #[test]
    fn password_and_totp_is_mfa_but_phishable() {
        let a = Authn::from_methods(&[Method::Password, Method::Totp], T);
        assert_eq!(a.amr, ["mfa", "otp", "pwd"]);
        assert_eq!(a.assurance(), Assurance::Aal2);
    }

    /// ADR-008: email is never the basis for `mfa`, whatever it is paired with.
    #[test]
    fn an_email_code_never_counts_as_a_second_factor() {
        for methods in [
            vec![Method::Email],
            vec![Method::Password, Method::Email],
            vec![Method::Email, Method::Email],
        ] {
            let a = Authn::from_methods(&methods, T);
            assert!(!a.has("mfa"), "{methods:?} must not yield mfa");
            assert_eq!(a.assurance(), Assurance::Aal1);
            assert!(a.has("email"));
            assert!(!a.has("otp"), "an email code is not reported as TOTP");
        }
        // email + TOTP is MFA because of the TOTP, exactly as password + TOTP is.
        let a = Authn::from_methods(&[Method::Email, Method::Totp], T);
        assert!(a.has("mfa"));
        assert_eq!(a.assurance(), Assurance::Aal2);
    }

    #[test]
    fn a_verified_passkey_is_phishing_resistant_mfa_on_its_own() {
        let synced = Authn::from_methods(&[passkey(true)], T);
        assert_eq!(synced.amr, ["mfa", "swk", "user"]);
        assert_eq!(synced.assurance(), Assurance::Aal2PhishingResistant);
        let bound = Authn::from_methods(&[passkey(false)], T);
        assert_eq!(bound.amr, ["hwk", "mfa", "user"]);
        let with_password = Authn::from_methods(&[Method::Password, passkey(true)], T);
        assert_eq!(with_password.amr, ["mfa", "pwd", "swk", "user"]);
        assert_eq!(with_password.assurance(), Assurance::Aal2PhishingResistant);
    }

    #[test]
    fn a_passkey_without_user_verification_is_one_factor() {
        let a = Authn::from_methods(
            &[Method::Passkey {
                user_verified: false,
                synced: false,
            }],
            T,
        );
        assert_eq!(a.amr, ["hwk"]);
        assert_eq!(a.assurance(), Assurance::Aal1);
    }

    #[test]
    fn a_recovery_code_is_aal2_never_phishing_resistant() {
        let a = Authn::from_methods(&[Method::Password, Method::RecoveryCode], T);
        assert_eq!(a.amr, ["mfa", "pwd", "rc"]);
        assert_eq!(a.assurance(), Assurance::Aal2);
        assert!(a.assurance() < Assurance::Aal2PhishingResistant);
    }

    #[test]
    fn sms_is_flagged_and_federated_is_its_own_category() {
        let a = Authn::from_methods(&[Method::Password, Method::Sms], T);
        assert_eq!(a.amr, ["mfa", "pwd", "sms"]);
        let f = Authn::from_methods(&[Method::Federated], T);
        assert_eq!(f.amr, ["fed"]);
        assert_eq!(f.assurance(), Assurance::Aal1);
        let f2 = Authn::from_methods(&[Method::Federated, Method::Totp], T);
        assert!(f2.has("mfa"));
    }

    #[test]
    fn unknown_acr_reads_as_the_weakest() {
        assert_eq!(Assurance::from_acr("urn:other:gold"), Assurance::Aal1);
        assert_eq!(Assurance::from_acr(ACR_AAL2), Assurance::Aal2);
    }

    #[test]
    fn round_trips_through_claims() {
        let a = Authn::from_methods(&[Method::Password, Method::Totp], T);
        let mut payload = serde_json::json!({"sub": "u"});
        a.write_into(&mut payload);
        assert_eq!(Authn::from_claims(&payload), Some(a));
        assert_eq!(Authn::from_claims(&serde_json::json!({"sub": "u"})), None);
    }
}
