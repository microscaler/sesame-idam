//! Conversion from BRRTRouter-validated Sesame claims to Lifeguard [`SessionContext`].
//!
//! Input must be JWT claims already authenticated by `BRRTRouter`. Raw bearer payloads
//! and unvalidated headers are never accepted as identity sources.

use lifeguard::SessionContext;
use serde_json::Value;
use uuid::Uuid;

/// Failure building database session context from validated claims.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RlsContextError {
    MissingValidatedClaims,
    MissingField(&'static str),
    InvalidField(&'static str),
    ClaimMismatch {
        first: &'static str,
        second: &'static str,
    },
    TenantHeaderConflict,
}

impl std::fmt::Display for RlsContextError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingValidatedClaims => f.write_str("validated claims are missing"),
            Self::MissingField(field) => write!(f, "required claim is missing: {field}"),
            Self::InvalidField(field) => write!(f, "claim has an invalid type or value: {field}"),
            Self::ClaimMismatch { first, second } => {
                write!(f, "validated claims disagree: {first} and {second}")
            }
            Self::TenantHeaderConflict => {
                f.write_str("x-tenant-id conflicts with the validated tenant claim")
            }
        }
    }
}

impl std::error::Error for RlsContextError {}

fn required_string<'a>(value: &'a Value, field: &'static str) -> Result<&'a str, RlsContextError> {
    let string = value
        .as_str()
        .ok_or(RlsContextError::InvalidField(field))?
        .trim();
    if string.is_empty() {
        return Err(RlsContextError::InvalidField(field));
    }
    Ok(string)
}

fn required_uuid(value: &Value, field: &'static str) -> Result<Uuid, RlsContextError> {
    let raw = required_string(value, field)?;
    Uuid::parse_str(raw).map_err(|_| RlsContextError::InvalidField(field))
}

fn string_array(value: &Value, field: &'static str) -> Result<Vec<String>, RlsContextError> {
    let values = value
        .as_array()
        .ok_or(RlsContextError::InvalidField(field))?;
    values
        .iter()
        .map(|value| required_string(value, field).map(str::to_string))
        .collect()
}

/// Build Lifeguard session context from claims already validated by `BRRTRouter`.
///
/// An optional `X-Tenant-ID` value is only a cross-check; it cannot supply a missing tenant.
///
/// # Errors
///
/// Returns [`RlsContextError`] when required claims are missing, malformed, or inconsistent.
pub fn session_context_from_validated_claims(
    claims: &Value,
    expected_tenant_header: Option<&str>,
) -> Result<SessionContext, RlsContextError> {
    let tenant = required_string(
        claims
            .get("tenant_id")
            .ok_or(RlsContextError::MissingField("tenant_id"))?,
        "tenant_id",
    )?;
    let subject_id = required_uuid(
        claims
            .get("sub")
            .ok_or(RlsContextError::MissingField("sub"))?,
        "sub",
    )?;
    let user_id = required_uuid(
        claims
            .get("user_id")
            .ok_or(RlsContextError::MissingField("user_id"))?,
        "user_id",
    )?;
    if subject_id != user_id {
        return Err(RlsContextError::ClaimMismatch {
            first: "sub",
            second: "user_id",
        });
    }

    let organization_id = required_uuid(
        claims
            .get("org_id")
            .ok_or(RlsContextError::MissingField("org_id"))?,
        "org_id",
    )?;
    let session_id = required_string(
        claims
            .get("sid")
            .ok_or(RlsContextError::MissingField("sid"))?,
        "sid",
    )?;

    let sx = claims
        .get("https://sesame-idam.dev/claims")
        .and_then(Value::as_object)
        .ok_or(RlsContextError::MissingField(
            "https://sesame-idam.dev/claims",
        ))?;
    let authz_tenant = required_string(
        sx.get("tenant")
            .ok_or(RlsContextError::MissingField("sx.tenant"))?,
        "sx.tenant",
    )?;
    if tenant != authz_tenant {
        return Err(RlsContextError::ClaimMismatch {
            first: "tenant_id",
            second: "sx.tenant",
        });
    }

    if expected_tenant_header
        .map(str::trim)
        .filter(|header| !header.is_empty())
        .is_some_and(|header| header != tenant)
    {
        return Err(RlsContextError::TenantHeaderConflict);
    }

    let roles = string_array(
        sx.get("roles")
            .ok_or(RlsContextError::MissingField("sx.roles"))?,
        "sx.roles",
    )?;
    let permissions = string_array(
        sx.get("permissions")
            .ok_or(RlsContextError::MissingField("sx.permissions"))?,
        "sx.permissions",
    )?;
    let user_type = claims
        .get("user_type")
        .map(|value| required_string(value, "user_type").map(str::to_string))
        .transpose()?;
    let org_type = sx
        .get("org_type")
        .map(|value| required_string(value, "sx.org_type").map(str::to_string))
        .transpose()?;

    Ok(SessionContext {
        tenant_id: tenant.to_string(),
        subject_id,
        organization_id,
        session_id: session_id.to_string(),
        roles,
        permissions,
        user_type,
        org_type,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_claims() -> Value {
        serde_json::json!({
            "sub": "a1000001-0001-4000-8000-000000000001",
            "user_id": "a1000001-0001-4000-8000-000000000001",
            "sid": "session-1",
            "tenant_id": "tenant-alpha",
            "org_id": "b2000002-0002-4000-8000-000000000002",
            "user_type": "customer",
            "https://sesame-idam.dev/claims": {
                "tenant": "tenant-alpha",
                "roles": ["member"],
                "permissions": ["user:read"]
            }
        })
    }

    #[test]
    fn builds_complete_context() {
        let context =
            session_context_from_validated_claims(&sample_claims(), Some("tenant-alpha")).unwrap();
        assert_eq!(context.tenant_id, "tenant-alpha");
        assert_eq!(context.roles, ["member"]);
        assert_eq!(context.permissions, ["user:read"]);
        assert_eq!(context.user_type.as_deref(), Some("customer"));
    }

    #[test]
    fn rejects_missing_active_organization() {
        let mut claims = sample_claims();
        claims.as_object_mut().unwrap().remove("org_id");
        assert_eq!(
            session_context_from_validated_claims(&claims, None),
            Err(RlsContextError::MissingField("org_id"))
        );
    }

    #[test]
    fn rejects_subject_user_mismatch() {
        let mut claims = sample_claims();
        claims["user_id"] = Value::String("a1000001-0001-4000-8000-000000000099".to_string());
        assert!(matches!(
            session_context_from_validated_claims(&claims, None),
            Err(RlsContextError::ClaimMismatch {
                first: "sub",
                second: "user_id"
            })
        ));
    }

    #[test]
    fn rejects_namespaced_tenant_mismatch() {
        let mut claims = sample_claims();
        claims["https://sesame-idam.dev/claims"]["tenant"] =
            Value::String("tenant-beta".to_string());
        assert!(matches!(
            session_context_from_validated_claims(&claims, None),
            Err(RlsContextError::ClaimMismatch {
                first: "tenant_id",
                second: "sx.tenant"
            })
        ));
    }

    #[test]
    fn rejects_tenant_header_conflict() {
        assert_eq!(
            session_context_from_validated_claims(&sample_claims(), Some("tenant-beta")),
            Err(RlsContextError::TenantHeaderConflict)
        );
    }
}
