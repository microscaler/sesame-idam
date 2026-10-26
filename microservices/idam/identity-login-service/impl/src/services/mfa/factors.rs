//! Enrolled authenticators and recovery codes in Postgres (`sesame_idam.user_mfa_factors`,
//! `sesame_idam.user_recovery_codes`; migration `identity-login-service/20261027090000_mfa_factors.sql`).
//!
//! Every statement runs under the tenant-only RLS context ([`sesame_idam_database::with_pre_auth_tenant`]): the
//! tables force row-level security on `tenant_id`, and every query also names the tenant and the user, so a factor
//! is only ever read or changed for the account that owns it. Values cross as text (ids, JSON) and integers (unix
//! seconds), cast in SQL.

use lifeguard::may_postgres::types::ToSql;
use lifeguard::{LifeError, LifeExecutor};
use uuid::Uuid;

/// What kind of authenticator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Totp,
    Passkey,
}

impl Kind {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Totp => "totp",
            Self::Passkey => "passkey",
        }
    }

    fn parse(s: &str) -> Option<Self> {
        match s {
            "totp" => Some(Self::Totp),
            "passkey" => Some(Self::Passkey),
            _ => None,
        }
    }
}

/// One enrolled (or pending) authenticator.
#[derive(Debug, Clone)]
pub struct Factor {
    pub id: Uuid,
    pub user_id: Uuid,
    pub kind: Kind,
    /// Confirmed with a first code / a completed registration. Pending factors never satisfy a challenge.
    pub active: bool,
    pub label: String,
    pub totp_secret_sealed: Option<String>,
    pub totp_last_step: Option<i64>,
    pub credential_id: Option<String>,
    /// The webauthn-rs `Passkey` as JSON (public key, counter, flags; no secret).
    pub passkey: Option<String>,
    /// The passkey is backup-eligible (synced between devices).
    pub passkey_synced: Option<bool>,
    pub created_at: i64,
    pub last_used_at: Option<i64>,
}

const COLUMNS: &str =
    "id::text, user_id::text, kind, status, label, totp_secret_sealed, totp_last_step, \
     credential_id, passkey::text, passkey_synced, extract(epoch from created_at)::bigint, \
     extract(epoch from last_used_at)::bigint";

fn row_to_factor(row: &lifeguard::may_postgres::Row) -> Result<Factor, LifeError> {
    let err = |e: lifeguard::may_postgres::Error| LifeError::Other(format!("mfa factor row: {e}"));
    let id: String = row.try_get(0).map_err(err)?;
    let user_id: String = row.try_get(1).map_err(err)?;
    let kind: String = row.try_get(2).map_err(err)?;
    let status: String = row.try_get(3).map_err(err)?;
    Ok(Factor {
        id: Uuid::parse_str(&id).map_err(|e| LifeError::Other(e.to_string()))?,
        user_id: Uuid::parse_str(&user_id).map_err(|e| LifeError::Other(e.to_string()))?,
        kind: Kind::parse(&kind)
            .ok_or_else(|| LifeError::Other(format!("unknown factor kind {kind}")))?,
        active: status == "active",
        label: row.try_get(4).map_err(err)?,
        totp_secret_sealed: row.try_get(5).map_err(err)?,
        totp_last_step: row.try_get(6).map_err(err)?,
        credential_id: row.try_get(7).map_err(err)?,
        passkey: row.try_get(8).map_err(err)?,
        passkey_synced: row.try_get(9).map_err(err)?,
        created_at: row.try_get(10).map_err(err)?,
        last_used_at: row.try_get(11).map_err(err)?,
    })
}

fn in_tenant<T>(
    tenant_id: &str,
    f: impl FnOnce(&dyn LifeExecutor) -> Result<T, LifeError>,
) -> Result<T, LifeError> {
    sesame_idam_database::with_pre_auth_tenant(tenant_id, |exec| f(exec))
}

/// Every factor of a user, pending included, oldest first.
///
/// # Errors
/// Database failure.
pub fn list(tenant_id: &str, user_id: Uuid) -> Result<Vec<Factor>, LifeError> {
    let (t, u) = (tenant_id.to_string(), user_id.to_string());
    in_tenant(tenant_id, |exec| {
        exec.query_all(
            &format!(
                "SELECT {COLUMNS} FROM sesame_idam.user_mfa_factors \
                 WHERE tenant_id = $1 AND user_id = $2::uuid ORDER BY created_at"
            ),
            &[&t as &dyn ToSql, &u],
        )?
        .iter()
        .map(row_to_factor)
        .collect()
    })
}

/// The user's confirmed factors.
///
/// # Errors
/// Database failure.
pub fn list_active(tenant_id: &str, user_id: Uuid) -> Result<Vec<Factor>, LifeError> {
    Ok(list(tenant_id, user_id)?
        .into_iter()
        .filter(|f| f.active)
        .collect())
}

/// Start a TOTP enrolment: a pending factor (any earlier pending TOTP enrolment of the user is dropped). The sealed
/// secret is bound to `factor_id`, which the caller generated to seal it.
///
/// # Errors
/// Database failure.
pub fn insert_pending_totp(
    tenant_id: &str,
    user_id: Uuid,
    factor_id: Uuid,
    label: &str,
    sealed_secret: &str,
) -> Result<(), LifeError> {
    let (t, u, id, l, s) = (
        tenant_id.to_string(),
        user_id.to_string(),
        factor_id.to_string(),
        label.to_string(),
        sealed_secret.to_string(),
    );
    in_tenant(tenant_id, |exec| {
        exec.execute(
            "DELETE FROM sesame_idam.user_mfa_factors \
             WHERE tenant_id = $1 AND user_id = $2::uuid AND kind = 'totp' AND status = 'pending'",
            &[&t as &dyn ToSql, &u],
        )?;
        exec.execute(
            "INSERT INTO sesame_idam.user_mfa_factors (id, tenant_id, user_id, kind, status, label, totp_secret_sealed) \
             VALUES ($1::uuid, $2, $3::uuid, 'totp', 'pending', $4, $5)",
            &[&id as &dyn ToSql, &t, &u, &l, &s],
        )?;
        Ok(())
    })
}

/// Confirm a pending TOTP enrolment with the step its first code matched. `false` when there is no such pending
/// factor (or the user already has an active authenticator app: one per account).
///
/// # Errors
/// Database failure other than the one-app constraint.
pub fn confirm_totp(
    tenant_id: &str,
    user_id: Uuid,
    factor_id: Uuid,
    step: i64,
) -> Result<bool, LifeError> {
    let (t, u, id) = (
        tenant_id.to_string(),
        user_id.to_string(),
        factor_id.to_string(),
    );
    let result = in_tenant(tenant_id, |exec| {
        exec.execute(
            "UPDATE sesame_idam.user_mfa_factors \
             SET status = 'active', confirmed_at = now(), last_used_at = now(), totp_last_step = $4 \
             WHERE tenant_id = $1 AND user_id = $2::uuid AND id = $3::uuid AND kind = 'totp' AND status = 'pending'",
            &[&t as &dyn ToSql, &u, &id, &step],
        )
    });
    match result {
        Ok(n) => Ok(n == 1),
        Err(e) if e.to_string().contains("uq_user_mfa_factors_one_totp") => Ok(false),
        Err(e) => Err(e),
    }
}

/// Record a TOTP code's step as used, only if it is later than the last one: the replay check and the update are one
/// statement, so two concurrent uses of one code cannot both succeed. `true` when this use won.
///
/// # Errors
/// Database failure.
pub fn advance_totp_step(
    tenant_id: &str,
    user_id: Uuid,
    factor_id: Uuid,
    step: i64,
) -> Result<bool, LifeError> {
    let (t, u, id) = (
        tenant_id.to_string(),
        user_id.to_string(),
        factor_id.to_string(),
    );
    in_tenant(tenant_id, |exec| {
        exec.execute(
            "UPDATE sesame_idam.user_mfa_factors SET totp_last_step = $4, last_used_at = now() \
             WHERE tenant_id = $1 AND user_id = $2::uuid AND id = $3::uuid AND status = 'active' \
               AND (totp_last_step IS NULL OR totp_last_step < $4)",
            &[&t as &dyn ToSql, &u, &id, &step],
        )
        .map(|n| n == 1)
    })
}

/// Store a newly registered passkey (active at once: the registration ceremony is its confirmation).
///
/// # Errors
/// Database failure, including a credential id already registered in the tenant.
pub fn insert_passkey(
    tenant_id: &str,
    user_id: Uuid,
    label: &str,
    credential_id: &str,
    passkey_json: &str,
    synced: bool,
) -> Result<Uuid, LifeError> {
    let id = Uuid::new_v4();
    let row: [String; 6] = [
        id.to_string(),
        tenant_id.to_string(),
        user_id.to_string(),
        label.to_string(),
        credential_id.to_string(),
        passkey_json.to_string(),
    ];
    in_tenant(tenant_id, |exec| {
        exec.execute(
            "INSERT INTO sesame_idam.user_mfa_factors \
             (id, tenant_id, user_id, kind, status, label, credential_id, passkey, passkey_synced, confirmed_at) \
             VALUES ($1::uuid, $2, $3::uuid, 'passkey', 'active', $4, $5, $6::jsonb, $7, now())",
            &[&row[0] as &dyn ToSql, &row[1], &row[2], &row[3], &row[4], &row[5], &synced],
        )?;
        Ok(id)
    })
}

/// The active passkey with this credential id in the tenant (username-less sign-in), whoever owns it.
///
/// # Errors
/// Database failure.
pub fn find_passkey(tenant_id: &str, credential_id: &str) -> Result<Option<Factor>, LifeError> {
    let (t, c) = (tenant_id.to_string(), credential_id.to_string());
    in_tenant(tenant_id, |exec| {
        exec.query_all(
            &format!(
                "SELECT {COLUMNS} FROM sesame_idam.user_mfa_factors \
                 WHERE tenant_id = $1 AND credential_id = $2 AND kind = 'passkey' AND status = 'active'"
            ),
            &[&t as &dyn ToSql, &c],
        )?
        .first()
        .map(row_to_factor)
        .transpose()
    })
}

/// After a passkey sign-in: its updated credential (counter, backup state) and last use.
///
/// # Errors
/// Database failure.
pub fn touch_passkey(
    tenant_id: &str,
    user_id: Uuid,
    factor_id: Uuid,
    passkey_json: Option<&str>,
) -> Result<(), LifeError> {
    let (t, u, id) = (
        tenant_id.to_string(),
        user_id.to_string(),
        factor_id.to_string(),
    );
    let p = passkey_json.map(str::to_string);
    in_tenant(tenant_id, |exec| {
        exec.execute(
            "UPDATE sesame_idam.user_mfa_factors \
             SET last_used_at = now(), passkey = COALESCE($4::jsonb, passkey) \
             WHERE tenant_id = $1 AND user_id = $2::uuid AND id = $3::uuid",
            &[&t as &dyn ToSql, &u, &id, &p],
        )?;
        Ok(())
    })
}

/// Remove one factor of the user. `true` when it existed.
///
/// # Errors
/// Database failure.
pub fn remove(tenant_id: &str, user_id: Uuid, factor_id: Uuid) -> Result<bool, LifeError> {
    let (t, u, id) = (
        tenant_id.to_string(),
        user_id.to_string(),
        factor_id.to_string(),
    );
    in_tenant(tenant_id, |exec| {
        exec.execute(
            "DELETE FROM sesame_idam.user_mfa_factors WHERE tenant_id = $1 AND user_id = $2::uuid AND id = $3::uuid",
            &[&t as &dyn ToSql, &u, &id],
        )
        .map(|n| n == 1)
    })
}

/// Rename one factor. `true` when it existed.
///
/// # Errors
/// Database failure.
pub fn rename(
    tenant_id: &str,
    user_id: Uuid,
    factor_id: Uuid,
    label: &str,
) -> Result<bool, LifeError> {
    let (t, u, id, l) = (
        tenant_id.to_string(),
        user_id.to_string(),
        factor_id.to_string(),
        label.to_string(),
    );
    in_tenant(tenant_id, |exec| {
        exec.execute(
            "UPDATE sesame_idam.user_mfa_factors SET label = $4 \
             WHERE tenant_id = $1 AND user_id = $2::uuid AND id = $3::uuid",
            &[&t as &dyn ToSql, &u, &id, &l],
        )
        .map(|n| n == 1)
    })
}

/// Replace the user's recovery codes with a new set (stored as hashes; the old set stops working).
///
/// # Errors
/// Database failure.
pub fn replace_recovery_codes(
    tenant_id: &str,
    user_id: Uuid,
    hashes: &[String],
) -> Result<(), LifeError> {
    let (t, u) = (tenant_id.to_string(), user_id.to_string());
    in_tenant(tenant_id, |exec| {
        exec.execute(
            "DELETE FROM sesame_idam.user_recovery_codes WHERE tenant_id = $1 AND user_id = $2::uuid",
            &[&t as &dyn ToSql, &u],
        )?;
        for h in hashes {
            exec.execute(
                "INSERT INTO sesame_idam.user_recovery_codes (tenant_id, user_id, code_hash) VALUES ($1, $2::uuid, $3)",
                &[&t as &dyn ToSql, &u, h],
            )?;
        }
        Ok(())
    })
}

/// Use a recovery code: marks it used if it is one of the user's unused codes. `true` when it was.
///
/// # Errors
/// Database failure.
pub fn consume_recovery_code(
    tenant_id: &str,
    user_id: Uuid,
    code_hash: &str,
) -> Result<bool, LifeError> {
    let (t, u, h) = (
        tenant_id.to_string(),
        user_id.to_string(),
        code_hash.to_string(),
    );
    in_tenant(tenant_id, |exec| {
        exec.execute(
            "UPDATE sesame_idam.user_recovery_codes SET used_at = now() \
             WHERE tenant_id = $1 AND user_id = $2::uuid AND code_hash = $3 AND used_at IS NULL",
            &[&t as &dyn ToSql, &u, &h],
        )
        .map(|n| n == 1)
    })
}

/// How many unused recovery codes the user has.
///
/// # Errors
/// Database failure.
pub fn recovery_codes_left(tenant_id: &str, user_id: Uuid) -> Result<i64, LifeError> {
    let (t, u) = (tenant_id.to_string(), user_id.to_string());
    in_tenant(tenant_id, |exec| {
        let row = exec.query_one(
            "SELECT count(*) FROM sesame_idam.user_recovery_codes \
             WHERE tenant_id = $1 AND user_id = $2::uuid AND used_at IS NULL",
            &[&t as &dyn ToSql, &u],
        )?;
        row.try_get::<_, i64>(0)
            .map_err(|e| LifeError::Other(e.to_string()))
    })
}

/// Remove every factor and recovery code of the user (support-mediated reset). Returns how many factors went.
///
/// # Errors
/// Database failure.
pub fn remove_all(tenant_id: &str, user_id: Uuid) -> Result<u64, LifeError> {
    let (t, u) = (tenant_id.to_string(), user_id.to_string());
    in_tenant(tenant_id, |exec| {
        let n = exec.execute(
            "DELETE FROM sesame_idam.user_mfa_factors WHERE tenant_id = $1 AND user_id = $2::uuid",
            &[&t as &dyn ToSql, &u],
        )?;
        exec.execute(
            "DELETE FROM sesame_idam.user_recovery_codes WHERE tenant_id = $1 AND user_id = $2::uuid",
            &[&t as &dyn ToSql, &u],
        )?;
        Ok(n)
    })
}
