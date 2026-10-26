//! Which second factor an account must use (ADR-008 §2, minimal): organisation admins can require members to sign in
//! with any second factor or with a passkey (`sesame_idam.org_auth_policies`, set through org-mgmt). The strictest
//! policy among the user's active memberships applies — one person, one way in. No policy: optional (the tenant
//! default; retail users are not affected).

use lifeguard::may_postgres::types::ToSql;
use lifeguard::{LifeError, LifeExecutor};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use sesame_common::authn::{Assurance, Authn};

/// What a policy requires. Ordered: a later variant implies the earlier ones.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Requirement {
    /// A second factor is optional.
    #[default]
    None,
    /// Any second factor (TOTP, passkey, or a recovery code standing in for one).
    Mfa,
    /// A passkey: phishing-resistant.
    Passkey,
}

impl Requirement {
    #[must_use]
    pub fn parse(s: &str) -> Self {
        match s {
            "mfa" => Self::Mfa,
            "passkey" => Self::Passkey,
            _ => Self::None,
        }
    }

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Mfa => "mfa",
            Self::Passkey => "passkey",
        }
    }

    /// The least assurance a sign-in must reach.
    #[must_use]
    pub const fn min_assurance(self) -> Assurance {
        match self {
            Self::None => Assurance::Aal1,
            Self::Mfa => Assurance::Aal2,
            Self::Passkey => Assurance::Aal2PhishingResistant,
        }
    }

    /// Whether a ceremony that produced `authn` meets this requirement.
    #[must_use]
    pub fn satisfied_by(self, authn: &Authn) -> bool {
        authn.assurance() >= self.min_assurance()
    }
}

/// The strictest requirement among the user's active organisation memberships in the tenant.
///
/// # Errors
/// Database failure (callers fail closed: a sign-in whose policy cannot be read is not completed).
pub fn required_for(tenant_id: &str, user_id: Uuid) -> Result<Requirement, LifeError> {
    let (t, u) = (tenant_id.to_string(), user_id.to_string());
    sesame_idam_database::with_pre_auth_tenant(tenant_id, |exec| {
        let rows = (exec as &dyn LifeExecutor).query_all(
            "SELECT p.require_method FROM sesame_idam.org_auth_policies p \
             JOIN sesame_idam.org_memberships m ON m.org_id = p.org_id \
             WHERE p.tenant_id = $1 AND m.user_id = $2::uuid AND m.status = 'active'",
            &[&t as &dyn ToSql, &u],
        )?;
        let mut strictest = Requirement::None;
        for row in &rows {
            let value: String = row
                .try_get(0)
                .map_err(|e| LifeError::Other(e.to_string()))?;
            strictest = strictest.max(Requirement::parse(&value));
        }
        Ok(strictest)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use sesame_common::authn::Method;

    #[test]
    fn requirements_are_ordered_and_checked_against_assurance() {
        assert!(Requirement::Passkey > Requirement::Mfa);
        let pwd = Authn::from_methods(&[Method::Password], 0);
        let totp = Authn::from_methods(&[Method::Password, Method::Totp], 0);
        let rc = Authn::from_methods(&[Method::Password, Method::RecoveryCode], 0);
        let pk = Authn::from_methods(
            &[Method::Passkey {
                user_verified: true,
                synced: true,
            }],
            0,
        );
        assert!(Requirement::None.satisfied_by(&pwd));
        assert!(!Requirement::Mfa.satisfied_by(&pwd));
        assert!(Requirement::Mfa.satisfied_by(&totp));
        assert!(Requirement::Mfa.satisfied_by(&rc));
        assert!(!Requirement::Passkey.satisfied_by(&totp));
        assert!(
            !Requirement::Passkey.satisfied_by(&rc),
            "recovery never stands in for a required passkey"
        );
        assert!(Requirement::Passkey.satisfied_by(&pk));
    }

    #[test]
    fn unknown_stored_values_require_nothing_more_than_none() {
        assert_eq!(Requirement::parse("bogus"), Requirement::None);
        assert_eq!(Requirement::parse("passkey"), Requirement::Passkey);
    }
}
