//! Tenant SMS configuration: read, upsert, revoke (ADR-009 Phase 2).
//!
//! # Write-only credentials
//!
//! An auth token goes IN through [`upsert_envelope`] and only ever comes out
//! inside [`resolve_credential`] at send time. Nothing in this module returns
//! it to an API caller — the tenant console shows "configured ✓ / last
//! validated", never the value. A credential a UI can read back is a
//! credential an XSS can steal.
//!
//! # Trust is earned
//!
//! New/updated credentials land as `pending_validation`. Only a successful
//! live check promotes them to `active`, and only `active` configs resolve
//! for sending — so a typo'd token fails closed (falls back to email) rather
//! than silently burning sends.

use chrono::Utc;
use lifeguard::{ColumnTrait, LifeError, LifeExecutor, LifeModelTrait};
use lifeguard::active_model::ActiveModelTrait;
use uuid::Uuid;

use crate::models::tenant_sms_config::{
    Column, Entity, TenantSmsConfigModel, TenantSmsConfigRecord, CUSTODY_CONNECT, CUSTODY_ENVELOPE,
    STATUS_ACTIVE, STATUS_PENDING_VALIDATION, STATUS_REVOKED,
};
use crate::services::envelope::{self, Sealed};

/// Default per-tenant daily ceiling (cents) for a newly created config.
const DEFAULT_TENANT_CEILING_CENTS: i32 = 500;

/// Non-secret fields a caller may set, on either custody path.
///
/// Every field is "absent means keep current" so the console can edit a from
/// number without re-sending the credential.
#[derive(Debug, Clone, Default, serde::Deserialize)]
pub struct SmsConfigInput {
    pub messaging_service_sid: Option<String>,
    pub from_number: Option<String>,
    pub campaign_ref: Option<String>,
    pub daily_spend_ceiling_cents: Option<i32>,
}

impl SmsConfigInput {
    #[allow(clippy::unused_self)]
    fn or_existing(&self, incoming: Option<String>, existing: Option<String>) -> Option<String> {
        incoming.or(existing)
    }

    fn ceiling_or(&self, existing: Option<&TenantSmsConfigModel>) -> i32 {
        self.daily_spend_ceiling_cents
            .or_else(|| existing.map(|c| c.daily_spend_ceiling_cents))
            .unwrap_or(DEFAULT_TENANT_CEILING_CENTS)
    }
}

/// An explicit SQL `NULL` for a text column.
///
/// # Why this is needed
///
/// Lifeguard's generated `set_<field>(None)` marks the field as *unset*, and
/// `update()` only emits SET clauses for fields that are set — so passing
/// `None` leaves the existing column value in place. That is the right
/// default for partial updates, and exactly the wrong behaviour when the
/// point of the write is to destroy a secret: we would report the credential
/// as cleared while the ciphertext was still sitting in the row. The
/// `set_<field>_expr` escape hatch emits a real `= NULL`.
fn sql_null() -> sea_query::SimpleExpr {
    sea_query::Expr::val(sea_query::Value::String(None))
}

/// Clear every column that carries sealed credential material.
///
/// Only meaningful on UPDATE — the expression setters are rejected by
/// `insert()`, and on INSERT an unset column is already NULL.
fn clear_sealed(record: &mut TenantSmsConfigRecord) {
    record.set_auth_token_ciphertext_expr(sql_null());
    record.set_auth_token_nonce_expr(sql_null());
    record.set_dek_wrapped_expr(sql_null());
}

pub struct TenantSmsService;

/// What the console may safely see. Note the absence of any secret.
#[derive(Debug, Clone, serde::Serialize)]
pub struct SmsConfigView {
    pub tenant_id: String,
    pub environment: String,
    pub provider: String,
    pub custody_mode: String,
    pub status: String,
    /// `true` when a credential is stored — never the credential itself.
    pub credential_configured: bool,
    /// Non-secret sender identity, safe to display.
    pub account_sid: Option<String>,
    pub connected_account_sid: Option<String>,
    pub messaging_service_sid: Option<String>,
    pub from_number: Option<String>,
    pub campaign_ref: Option<String>,
    pub daily_spend_ceiling_cents: i32,
    pub last_validated_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl From<&TenantSmsConfigModel> for SmsConfigView {
    fn from(c: &TenantSmsConfigModel) -> Self {
        Self {
            tenant_id: c.tenant_id.clone(),
            environment: c.environment.clone(),
            provider: c.provider.clone(),
            custody_mode: c.custody_mode.clone(),
            status: c.status.clone(),
            credential_configured: c.auth_token_ciphertext.is_some()
                || c.connected_account_sid.is_some(),
            account_sid: c.account_sid.clone(),
            connected_account_sid: c.connected_account_sid.clone(),
            messaging_service_sid: c.messaging_service_sid.clone(),
            from_number: c.from_number.clone(),
            campaign_ref: c.campaign_ref.clone(),
            daily_spend_ceiling_cents: c.daily_spend_ceiling_cents,
            last_validated_at: c.last_validated_at,
        }
    }
}

impl TenantSmsService {
    /// Fetch the config for a `(tenant, environment)`.
    ///
    /// # Errors
    ///
    /// Returns [`LifeError`] on query failure.
    pub fn find<E: LifeExecutor>(
        tenant_id: &str,
        environment: &str,
        exec: &E,
    ) -> Result<Option<TenantSmsConfigModel>, LifeError> {
        Entity::find()
            .filter(Column::TenantId.eq(tenant_id.to_string()))
            .filter(Column::Environment.eq(environment.to_string()))
            .find_one(exec)
    }

    /// Store (or replace) envelope-custody credentials.

    /// `auth_token: None` on an existing config means "keep what is stored" —
    /// the console never round-trips the secret just to edit a from number. The token is sealed
    /// before it touches the database and the config drops back to
    /// `pending_validation` until a live check passes.
    ///
    /// # Errors
    ///
    /// Returns [`LifeError`] when sealing fails or the write fails.
    pub fn upsert_envelope<E: LifeExecutor>(
        tenant_id: &str,
        environment: &str,
        account_sid: &str,
        auth_token: Option<&str>,
        opts: &SmsConfigInput,
        exec: &E,
    ) -> Result<(), LifeError> {
        let existing = Self::find(tenant_id, environment, exec)?;

        // A new token is sealed now; no token means keep the stored material
        // (and keep the existing trust status rather than re-validating).
        let (sealed, rotated) = match auth_token {
            Some(token) => {
                let sealed = envelope::encrypt(token)
                    .map_err(|e| LifeError::Other(format!("seal tenant credential: {e}")))?;
                (Some(sealed), true)
            }
            None => (None, false),
        };
        if sealed.is_none()
            && existing
                .as_ref()
                .is_none_or(|c| c.auth_token_ciphertext.is_none())
        {
            return Err(LifeError::Other(
                "auth_token is required when no credential is stored".to_string(),
            ));
        }

        let now = Utc::now();
        let mut record = TenantSmsConfigRecord::new();
        record
            .set_id(existing.as_ref().map_or_else(Uuid::new_v4, |c| c.id))
            .set_tenant_id(tenant_id.to_string())
            .set_environment(environment.to_string())
            .set_provider("twilio".to_string())
            .set_custody_mode(CUSTODY_ENVELOPE.to_string())
            .set_account_sid(Some(account_sid.to_string()))
            .set_auth_token_ciphertext(match &sealed {
                Some(s) => Some(s.ciphertext.clone()),
                None => existing.as_ref().and_then(|c| c.auth_token_ciphertext.clone()),
            })
            .set_auth_token_nonce(match &sealed {
                Some(s) => Some(s.nonce.clone()),
                None => existing.as_ref().and_then(|c| c.auth_token_nonce.clone()),
            })
            .set_dek_wrapped(match &sealed {
                Some(s) => Some(s.dek_wrapped.clone()),
                None => existing.as_ref().and_then(|c| c.dek_wrapped.clone()),
            })
            .set_messaging_service_sid(opts.or_existing(
                opts.messaging_service_sid.clone(),
                existing.as_ref().and_then(|c| c.messaging_service_sid.clone()),
            ))
            .set_from_number(opts.or_existing(
                opts.from_number.clone(),
                existing.as_ref().and_then(|c| c.from_number.clone()),
            ))
            .set_campaign_ref(opts.or_existing(
                opts.campaign_ref.clone(),
                existing.as_ref().and_then(|c| c.campaign_ref.clone()),
            ))
            .set_daily_spend_ceiling_cents(opts.ceiling_or(existing.as_ref()))
            // A rotated credential is never trusted on sight; an untouched
            // one keeps whatever trust it already earned.
            .set_status(if rotated {
                STATUS_PENDING_VALIDATION.to_string()
            } else {
                existing.as_ref().map_or_else(
                    || STATUS_PENDING_VALIDATION.to_string(),
                    |c| c.status.clone(),
                )
            })
            .set_last_validated_at(if rotated {
                None
            } else {
                existing.as_ref().and_then(|c| c.last_validated_at)
            })
            .set_created_at(existing.as_ref().map_or(now, |c| c.created_at))
            .set_updated_at(now);
        // Envelope custody supersedes any Connect authorisation. Only on
        // UPDATE: an unset column already inserts as NULL.
        if existing.is_some() {
            record.set_connected_account_sid_expr(sql_null());
        }

        Self::write(record, existing.is_some(), exec)
    }

    /// Store a Twilio Connect authorization (no secret held by Sesame).
    ///
    /// # Errors
    ///
    /// Returns [`LifeError`] on write failure.
    pub fn upsert_connect<E: LifeExecutor>(
        tenant_id: &str,
        environment: &str,
        connected_account_sid: &str,
        opts: &SmsConfigInput,
        exec: &E,
    ) -> Result<(), LifeError> {
        let existing = Self::find(tenant_id, environment, exec)?;
        let now = Utc::now();
        let mut record = TenantSmsConfigRecord::new();
        record
            .set_id(existing.as_ref().map_or_else(Uuid::new_v4, |c| c.id))
            .set_tenant_id(tenant_id.to_string())
            .set_environment(environment.to_string())
            .set_provider("twilio".to_string())
            .set_custody_mode(CUSTODY_CONNECT.to_string())
            .set_connected_account_sid(Some(connected_account_sid.to_string()))
            .set_messaging_service_sid(opts.or_existing(
                opts.messaging_service_sid.clone(),
                existing.as_ref().and_then(|c| c.messaging_service_sid.clone()),
            ))
            .set_from_number(opts.or_existing(
                opts.from_number.clone(),
                existing.as_ref().and_then(|c| c.from_number.clone()),
            ))
            .set_campaign_ref(opts.or_existing(
                opts.campaign_ref.clone(),
                existing.as_ref().and_then(|c| c.campaign_ref.clone()),
            ))
            .set_daily_spend_ceiling_cents(opts.ceiling_or(existing.as_ref()))
            .set_status(STATUS_PENDING_VALIDATION.to_string())
            .set_last_validated_at(None)
            .set_created_at(existing.as_ref().map_or(now, |c| c.created_at))
            .set_updated_at(now);
        // Connect supersedes any previously stored envelope material —
        // switching custody must not leave a sealed secret behind.
        if existing.is_some() {
            record.set_account_sid_expr(sql_null());
            clear_sealed(&mut record);
        }

        Self::write(record, existing.is_some(), exec)
    }

    fn write<E: LifeExecutor>(
        mut record: TenantSmsConfigRecord,
        update: bool,
        exec: &E,
    ) -> Result<(), LifeError> {
        if update {
            record
                .update(exec)
                .map_err(|e| LifeError::Other(e.to_string()))?;
        } else {
            record
                .insert(exec)
                .map_err(|e| LifeError::Other(e.to_string()))?;
        }
        Ok(())
    }

    /// Promote a config to `active` after a successful live validation.
    ///
    /// # Errors
    ///
    /// Returns [`LifeError`] when the config is missing or the write fails.
    pub fn mark_validated<E: LifeExecutor>(
        tenant_id: &str,
        environment: &str,
        exec: &E,
    ) -> Result<(), LifeError> {
        Self::set_status(tenant_id, environment, STATUS_ACTIVE, true, false, exec)
    }

    /// Revoke a config — sending stops immediately and email fallback applies.
    ///
    /// The sealed material is cleared as well as the status flipped: a
    /// revoked credential should be unusable even if the status check were
    /// ever bypassed. The row survives so the revocation stays auditable.
    ///
    /// # Errors
    ///
    /// Returns [`LifeError`] when the config is missing or the write fails.
    pub fn revoke<E: LifeExecutor>(
        tenant_id: &str,
        environment: &str,
        exec: &E,
    ) -> Result<(), LifeError> {
        Self::set_status(tenant_id, environment, STATUS_REVOKED, false, true, exec)
    }

    fn set_status<E: LifeExecutor>(
        tenant_id: &str,
        environment: &str,
        status: &str,
        stamp_validated: bool,
        clear_credential: bool,
        exec: &E,
    ) -> Result<(), LifeError> {
        let existing = Self::find(tenant_id, environment, exec)?
            .ok_or_else(|| LifeError::Other(format!("no sms config for {tenant_id}/{environment}")))?;
        let now = Utc::now();
        let mut record = TenantSmsConfigRecord::new();
        record
            .set_id(existing.id)
            .set_tenant_id(existing.tenant_id.clone())
            .set_environment(existing.environment.clone())
            .set_provider(existing.provider.clone())
            .set_custody_mode(existing.custody_mode.clone())
            .set_connected_account_sid(existing.connected_account_sid.clone())
            .set_account_sid(existing.account_sid.clone())
            .set_auth_token_ciphertext(existing.auth_token_ciphertext.clone())
            .set_auth_token_nonce(existing.auth_token_nonce.clone())
            .set_dek_wrapped(existing.dek_wrapped.clone())
            .set_messaging_service_sid(existing.messaging_service_sid.clone())
            .set_from_number(existing.from_number.clone())
            .set_campaign_ref(existing.campaign_ref.clone())
            .set_daily_spend_ceiling_cents(existing.daily_spend_ceiling_cents)
            .set_status(status.to_string())
            .set_last_validated_at(if stamp_validated { Some(now) } else { existing.last_validated_at })
            .set_created_at(existing.created_at)
            .set_updated_at(now);
        if clear_credential {
            clear_sealed(&mut record);
            record.set_connected_account_sid_expr(sql_null());
        }
        record
            .update(exec)
            .map_err(|e| LifeError::Other(e.to_string()))?;
        Ok(())
    }

    /// Unseal the tenant's auth token for a send. ONLY for envelope custody,
    /// and ONLY for an `active` config.
    ///
    /// The plaintext lives in the returned `String` and nowhere else.
    #[must_use]
    pub fn resolve_credential(config: &TenantSmsConfigModel) -> Option<String> {
        if config.status != STATUS_ACTIVE || config.custody_mode != CUSTODY_ENVELOPE {
            return None;
        }
        let sealed = Sealed {
            ciphertext: config.auth_token_ciphertext.clone()?,
            nonce: config.auth_token_nonce.clone()?,
            dek_wrapped: config.dek_wrapped.clone()?,
        };
        match envelope::decrypt(&sealed) {
            Ok(token) => Some(token),
            Err(e) => {
                // Never log the material — only that it failed.
                tracing::error!(error = %e, tenant = %config.tenant_id, "tenant SMS credential could not be unsealed");
                None
            }
        }
    }
}
