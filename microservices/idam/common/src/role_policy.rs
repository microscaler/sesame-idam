//! Which roles a tenant administrator may grant or take away (authz-core's role administration).
//!
//! A role is a string a relying party interprets, so a careless grant can mean far more than the admin intended: a
//! relying party's database may treat a role as "the service itself" (PriceWhisperer's `pw_service`), and an admin role
//! makes its holder a tenant administrator. Three rules, all checked before anything is written:
//!
//! 1. **Reserved roles are never granted through the API.** Built in: `pw_service`, `service`, `system`, `root`,
//!    `superuser`; a deployment adds more in `SESAME_RESERVED_ROLES`. They exist for service identities, not people.
//! 2. **An allow-list, when the deployment sets one.** `SESAME_ASSIGNABLE_ROLES` (comma separated) is the complete set
//!    of roles the tenant's admins may grant or revoke; unset, any well-formed, unreserved role may be.
//! 3. **No grant above your own.** An administrator role (`owner`, `tenant_admin`, `SESAME_TENANT_ADMIN_ROLES`) is
//!    granted or taken away only by someone who holds that same role or `owner`; `owner` only by an owner.
//!
//! Role names compare case-insensitively everywhere (a grant of `Owner` is a grant of `owner`).

use crate::tenant_admin::{configured_admin_roles, ROLE_OWNER};

/// Roles kept for service identities, whatever the deployment says.
pub const RESERVED_ROLES: [&str; 5] = ["pw_service", "service", "system", "root", "superuser"];

/// Why a role change is refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refusal {
    /// A role reserved for service identities.
    Reserved,
    /// Not on the deployment's list of assignable roles.
    NotAssignable,
    /// An administrator role the caller does not hold themselves.
    AboveCaller,
}

impl Refusal {
    #[must_use]
    pub fn message(&self) -> &'static str {
        match self {
            Self::Reserved => {
                "That role is reserved for service identities and cannot be granted or revoked here"
            }
            Self::NotAssignable => {
                "That role is not one this deployment allows administrators to grant"
            }
            Self::AboveCaller => {
                "Only someone who holds that administrator role (or owner) may grant or revoke it"
            }
        }
    }
}

/// The deployment's policy (from the environment, read once per process).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RolePolicy {
    pub reserved: Vec<String>,
    pub assignable: Option<Vec<String>>,
    pub admin: Vec<String>,
}

fn list(raw: Option<&str>) -> Vec<String> {
    raw.unwrap_or("")
        .split(',')
        .map(|r| r.trim().to_ascii_lowercase())
        .filter(|r| !r.is_empty())
        .collect()
}

impl RolePolicy {
    /// From explicit values (tests); `assignable` None or empty means no allow-list.
    #[must_use]
    pub fn new(reserved_extra: Option<&str>, assignable: Option<&str>, admin: &[String]) -> Self {
        let mut reserved: Vec<String> = RESERVED_ROLES.iter().map(|r| (*r).to_string()).collect();
        reserved.extend(list(reserved_extra));
        let assignable = Some(list(assignable)).filter(|l| !l.is_empty());
        Self {
            reserved,
            assignable,
            admin: admin.iter().map(|r| r.to_ascii_lowercase()).collect(),
        }
    }

    /// The deployment's policy: SESAME_RESERVED_ROLES, SESAME_ASSIGNABLE_ROLES and the admin roles.
    #[must_use]
    pub fn configured() -> &'static Self {
        static P: std::sync::OnceLock<RolePolicy> = std::sync::OnceLock::new();
        P.get_or_init(|| {
            Self::new(
                std::env::var("SESAME_RESERVED_ROLES").ok().as_deref(),
                std::env::var("SESAME_ASSIGNABLE_ROLES").ok().as_deref(),
                configured_admin_roles(),
            )
        })
    }

    /// Is `role` an administrator role here?
    #[must_use]
    pub fn is_admin_role(&self, role: &str) -> bool {
        self.admin.contains(&role.to_ascii_lowercase())
    }

    /// May a caller holding `caller_roles` grant (or take away) `role`?
    ///
    /// # Errors
    /// The [`Refusal`] that applies, checked in the order of the module rules.
    pub fn may_change(&self, caller_roles: &[String], role: &str) -> Result<(), Refusal> {
        let role = role.trim().to_ascii_lowercase();
        if self.reserved.contains(&role) {
            return Err(Refusal::Reserved);
        }
        if let Some(allowed) = &self.assignable {
            if !allowed.contains(&role) && !self.admin.contains(&role) {
                return Err(Refusal::NotAssignable);
            }
        }
        if self.admin.contains(&role) {
            let holds = |r: &str| caller_roles.iter().any(|c| c.eq_ignore_ascii_case(r));
            let ok = if role == ROLE_OWNER {
                holds(ROLE_OWNER)
            } else {
                holds(&role) || holds(ROLE_OWNER)
            };
            if !ok {
                return Err(Refusal::AboveCaller);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn admin() -> Vec<String> {
        vec![
            "tenant_admin".into(),
            "owner".into(),
            "platform_admin".into(),
        ]
    }
    fn roles(r: &[&str]) -> Vec<String> {
        r.iter().map(|s| (*s).to_string()).collect()
    }

    #[test]
    fn reserved_roles_are_never_granted_whoever_asks() {
        let p = RolePolicy::new(Some("Billing_Service"), None, &admin());
        let owner = roles(&["owner"]);
        for r in [
            "pw_service",
            "PW_SERVICE",
            " service ",
            "system",
            "root",
            "superuser",
            "billing_service",
        ] {
            assert_eq!(p.may_change(&owner, r), Err(Refusal::Reserved), "{r}");
        }
    }

    #[test]
    fn the_allow_list_is_complete_when_set() {
        let p = RolePolicy::new(None, Some("crm.editor, crm.viewer"), &admin());
        let a = roles(&["platform_admin"]);
        assert!(p.may_change(&a, "crm.editor").is_ok());
        assert!(p.may_change(&a, "CRM.Viewer").is_ok());
        assert_eq!(
            p.may_change(&a, "anything_else"),
            Err(Refusal::NotAssignable)
        );
        // admin roles are governed by the ceiling rule, not the list
        assert!(p.may_change(&a, "platform_admin").is_ok());
        let unlisted = RolePolicy::new(None, Some(" "), &admin());
        assert!(
            unlisted.may_change(&a, "anything_else").is_ok(),
            "blank list = no list"
        );
    }

    #[test]
    fn nobody_grants_above_their_own_role() {
        let p = RolePolicy::new(None, None, &admin());
        let platform = roles(&["platform_admin"]);
        let tenant = roles(&["tenant_admin"]);
        let owner = roles(&["Owner"]);
        assert!(p.may_change(&platform, "platform_admin").is_ok());
        assert_eq!(
            p.may_change(&platform, "tenant_admin"),
            Err(Refusal::AboveCaller)
        );
        assert_eq!(p.may_change(&platform, "owner"), Err(Refusal::AboveCaller));
        assert_eq!(
            p.may_change(&tenant, "platform_admin"),
            Err(Refusal::AboveCaller)
        );
        assert_eq!(p.may_change(&tenant, "OWNER"), Err(Refusal::AboveCaller));
        for r in ["owner", "tenant_admin", "platform_admin", "crm.editor"] {
            assert!(p.may_change(&owner, r).is_ok(), "{r}");
        }
        assert!(
            p.may_change(&tenant, "crm.editor").is_ok(),
            "ordinary roles need only admin"
        );
    }
}
