//! Second factors and passkeys (ADR-008): the sign-in gate, the ceremonies and the factor store.
//!
//! # The gate
//!
//! Every way of proving who someone is (password, email code, magic link, SMS code, social provider, passkey) ends in
//! [`after_first_factor`] or, for the steps after it, [`next_step`]. It looks at what the ceremony has proved, the
//! factors the account has, and the policy of the account's organisations ([`policy`]), and either:
//!
//! * issues tokens (`200`, the usual `TokenResponse`, with truthful `amr`/`acr`/`auth_time`), or
//! * answers `202` with an `mfa_token` and what is needed next: a second factor (`methods`), or the enrolment of one
//!   (`enrollment_required`) when policy requires a method the account does not have.
//!
//! No token exists until the gate says so: a password alone never signs in an account that has a second factor, and
//! no first factor (an email code included) bypasses it.
//!
//! # Honest outcomes
//!
//! * The claims describe exactly the methods used ([`sesame_common::authn`]).
//! * A member who must enrol (organisation policy, no factor yet) is let in once enrolment completes, with the
//!   claims of what they actually proved (a password: `aal1`); every later sign-in must use the factor. Requiring an
//!   emailed code as well before enrolment is a policy decision left open (docs/DESIGN-mfa-passkeys.md §7).
//! * Recovery with a recovery code reaches `aal2`, never more: when policy requires a passkey, the user enrols a new
//!   one before any token is issued (ADR-008 §6).

pub mod context;
pub mod factors;
pub mod notify;
pub mod passkey;
pub mod policy;
pub mod recovery;
pub mod totp;

use brrtrouter::typed::HttpJson;
use serde_json::{json, Value};
use uuid::Uuid;

use sesame_common::authn::{Authn, Method};

use context::{MfaContext, Mode};
use factors::{Factor, Kind};
use policy::Requirement;

/// Who is signing in, and for what.
#[derive(Debug, Clone)]
pub struct Subject {
    pub tenant_id: String,
    pub user_id: Uuid,
    pub email: String,
    /// The registered client's portal (or `frontend` for flows without a client).
    pub portal: String,
    pub organization_id: Option<String>,
}

/// What the gate decided after the latest step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Next {
    /// The ceremony is complete: issue tokens with these claims.
    Issue(Authn),
    /// A second factor is needed: these are the methods that may be used.
    Challenge(Vec<&'static str>),
    /// Policy requires a method the account lacks: it must be enrolled first.
    Enroll(Requirement),
}

/// What the account has.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Holdings {
    pub totp: bool,
    pub passkey: bool,
    pub recovery_codes: bool,
}

impl Holdings {
    #[must_use]
    pub fn of(active: &[Factor], recovery_codes_left: i64) -> Self {
        Self {
            totp: active.iter().any(|f| f.kind == Kind::Totp),
            passkey: active.iter().any(|f| f.kind == Kind::Passkey),
            recovery_codes: recovery_codes_left > 0,
        }
    }

    #[must_use]
    pub fn any_factor(self) -> bool {
        self.totp || self.passkey
    }
}

/// The decision, from what was proved, what the account holds and what policy requires (pure; unit-tested).
#[must_use]
pub fn decide(methods: &[Method], auth_time: i64, held: Holdings, required: Requirement) -> Next {
    let authn = Authn::from_methods(methods, auth_time);
    let second_factor_used = methods.iter().any(|m| {
        matches!(
            m,
            Method::Totp | Method::RecoveryCode | Method::Passkey { .. } | Method::Sms
        )
    }) && authn.has("mfa");
    let passkey_used = matches!(
        authn.assurance(),
        sesame_common::authn::Assurance::Aal2PhishingResistant
    );

    // An account with a factor always uses one (a passkey sign-in already has).
    if held.any_factor() && !second_factor_used && !passkey_used {
        let mut offered = Vec::new();
        if required == Requirement::Passkey && held.passkey {
            offered.push("passkey");
        } else {
            if held.totp {
                offered.push("totp");
            }
            if held.passkey {
                offered.push("passkey");
            }
        }
        if held.recovery_codes {
            offered.push("recovery_code");
        }
        return Next::Challenge(offered);
    }
    if required.satisfied_by(&authn) {
        return Next::Issue(authn);
    }
    // Policy wants more than this ceremony can give with what the account holds.
    match required {
        Requirement::Passkey if held.passkey && !methods.contains(&Method::RecoveryCode) => {
            // Has a passkey but used something else: use the passkey.
            Next::Challenge(vec!["passkey"])
        }
        Requirement::Passkey => Next::Enroll(Requirement::Passkey),
        _ => Next::Enroll(required),
    }
}

/// Load what the account holds and its policy.
///
/// # Errors
/// Database failure (the caller fails closed).
pub fn load_state(
    tenant_id: &str,
    user_id: Uuid,
) -> Result<(Holdings, Requirement), lifeguard::LifeError> {
    let active = factors::list_active(tenant_id, user_id)?;
    let codes = if active.is_empty() {
        0
    } else {
        factors::recovery_codes_left(tenant_id, user_id)?
    };
    let required = policy::required_for(tenant_id, user_id)?;
    Ok((Holdings::of(&active, codes), required))
}

/// The gate after a first factor: tokens (200) or what comes next (202).
#[must_use]
pub fn after_first_factor(subject: &Subject, method: Method) -> HttpJson<Value> {
    let (held, required) = match load_state(&subject.tenant_id, subject.user_id) {
        Ok(v) => v,
        Err(e) => {
            tracing::error!(error = %e, "mfa: could not read the account's factors or policy");
            return unavailable();
        }
    };
    let ctx = MfaContext {
        mode: Mode::Challenge,
        tenant_id: subject.tenant_id.clone(),
        user_id: subject.user_id,
        email: subject.email.clone(),
        portal: subject.portal.clone(),
        organization_id: subject.organization_id.clone(),
        methods: vec![method],
        auth_time: chrono::Utc::now().timestamp(),
        required,
        attempts: 0,
    };
    match decide(&ctx.methods, ctx.auth_time, held, required) {
        Next::Issue(authn) => issue(&ctx, &authn),
        next => open_context(ctx, &next),
    }
}

/// The gate after a further step of an existing context (`token`): finish it and issue, or move it on.
#[must_use]
pub fn next_step(token: &str, mut ctx: MfaContext) -> HttpJson<Value> {
    let (held, required) = match load_state(&ctx.tenant_id, ctx.user_id) {
        Ok(v) => v,
        Err(e) => {
            tracing::error!(error = %e, "mfa: could not read the account's factors or policy");
            return unavailable();
        }
    };
    ctx.required = ctx.required.max(required);
    match decide(&ctx.methods, ctx.auth_time, held, ctx.required) {
        Next::Issue(authn) => finish_and_issue(token, &ctx, &authn),
        Next::Challenge(offered) => {
            ctx.mode = Mode::Challenge;
            if let Err(e) = context::update(token, &ctx) {
                tracing::error!(error = %e, "mfa: context update failed");
                return unavailable();
            }
            challenge_body(token, &ctx, &offered, None)
        }
        Next::Enroll(requirement) => {
            ctx.mode = Mode::Enroll;
            if let Err(e) = context::update(token, &ctx) {
                tracing::error!(error = %e, "mfa: context update failed");
                return unavailable();
            }
            challenge_body(token, &ctx, &[], Some(requirement))
        }
    }
}

/// End a context and issue tokens, once: of two concurrent completions only the one that deletes the context issues.
#[must_use]
pub fn finish_and_issue(token: &str, ctx: &MfaContext, authn: &Authn) -> HttpJson<Value> {
    match context::finish(token) {
        Ok(true) => issue(ctx, authn),
        Ok(false) => invalid_token(),
        Err(e) => {
            tracing::error!(error = %e, "mfa: context finish failed");
            unavailable()
        }
    }
}

fn open_context(mut ctx: MfaContext, next: &Next) -> HttpJson<Value> {
    let (offered, enroll) = match next {
        Next::Challenge(o) => (o.clone(), None),
        Next::Enroll(r) => {
            ctx.mode = Mode::Enroll;
            (vec![], Some(*r))
        }
        Next::Issue(_) => (vec![], None),
    };
    match context::create(&ctx) {
        Ok((token, _)) => challenge_body(&token, &ctx, &offered, enroll),
        Err(e) => {
            tracing::error!(error = %e, "mfa: could not store the sign-in context");
            unavailable()
        }
    }
}

fn challenge_body(
    token: &str,
    ctx: &MfaContext,
    offered: &[&str],
    enroll: Option<Requirement>,
) -> HttpJson<Value> {
    let expires_in = context::remaining(token).unwrap_or(context::SIGN_IN_TTL_SECS);
    let _ = ctx;
    HttpJson::new(
        202,
        json!({
            "mfa_required": true,
            "mfa_token": token,
            "expires_in": expires_in,
            "methods": offered,
            "enrollment_required": enroll.map(Requirement::as_str),
        }),
    )
}

/// Issue the session: authz enrichment, active organisation, tokens. The `TokenResponse` body (200).
#[must_use]
pub fn issue(ctx: &MfaContext, authn: &Authn) -> HttpJson<Value> {
    match crate::services::login_success::issue_session(
        &ctx.user_id.to_string(),
        &ctx.tenant_id,
        &ctx.portal,
        ctx.organization_id.as_deref(),
        authn,
    ) {
        Ok(body) => HttpJson::ok(body),
        Err(e) => {
            tracing::error!(error = %e, "mfa: token issuance failed");
            HttpJson::new(
                500,
                json!({"error": "internal_error", "error_description": "An unexpected error occurred"}),
            )
        }
    }
}

/// Recovery codes for an account that just got its first factor (none yet): generated, stored hashed, returned once.
#[must_use]
pub fn first_recovery_codes(tenant_id: &str, user_id: Uuid) -> Option<Vec<String>> {
    match factors::recovery_codes_left(tenant_id, user_id) {
        Ok(n) if n > 0 => return None,
        Ok(_) => {}
        Err(e) => {
            tracing::error!(error = %e, "mfa: could not count recovery codes");
            return None;
        }
    }
    regenerate_recovery_codes(tenant_id, user_id).ok()
}

/// A fresh set of recovery codes (replacing any old set).
///
/// # Errors
/// RNG or database failure.
pub fn regenerate_recovery_codes(tenant_id: &str, user_id: Uuid) -> anyhow::Result<Vec<String>> {
    let codes = recovery::generate()?;
    let hashes = codes
        .iter()
        .filter_map(|c| recovery::normalize(c))
        .map(|n| recovery::hash(tenant_id, &user_id, &n))
        .collect::<Vec<_>>();
    factors::replace_recovery_codes(tenant_id, user_id, &hashes)?;
    Ok(codes)
}

/// A factor as the API shows it (never its secret or public key).
#[must_use]
pub fn factor_json(f: &Factor) -> Value {
    json!({
        "id": f.id.to_string(),
        "kind": f.kind.as_str(),
        "label": f.label,
        "created_at": f.created_at,
        "last_used_at": f.last_used_at,
        "synced": f.passkey_synced,
    })
}

/// The context behind `token`, if it exists and is in one of `modes`; else the error to answer.
///
/// # Errors
/// `401 invalid_token` (absent, expired, completed, or another mode), `503` when the store is unreachable.
pub fn context_for(token: &str, modes: &[Mode]) -> Result<MfaContext, HttpJson<Value>> {
    match context::get(token) {
        Ok(Some(ctx)) if modes.contains(&ctx.mode) => Ok(ctx),
        Ok(_) => Err(invalid_token()),
        Err(e) => {
            tracing::error!(error = %e, "mfa: context store unavailable");
            Err(unavailable())
        }
    }
}

/// The methods the context may use for its next step (recomputed, so a client cannot pick an unoffered one).
///
/// # Errors
/// Database failure.
pub fn offered(ctx: &MfaContext) -> Result<Vec<&'static str>, lifeguard::LifeError> {
    let (held, required) = load_state(&ctx.tenant_id, ctx.user_id)?;
    Ok(
        match decide(
            &ctx.methods,
            ctx.auth_time,
            held,
            ctx.required.max(required),
        ) {
            Next::Challenge(o) => o,
            _ => vec![],
        },
    )
}

/// After a factor was enrolled in context `token`: the account's first recovery codes (when it had none), and — when
/// the enrolment was what a sign-in was waiting for (`enroll` mode) — the tokens, with the claims of what that
/// sign-in actually proved (the new factor was bound during it, so it adds no assurance to it).
#[must_use]
pub fn enrollment_done(token: &str, ctx: &MfaContext, factor: &Factor) -> HttpJson<Value> {
    let mut body = json!({ "factor": factor_json(factor) });
    if let Some(codes) = first_recovery_codes(&ctx.tenant_id, ctx.user_id) {
        body["recovery_codes"] = json!(codes);
    }
    if ctx.mode == Mode::Enroll {
        let authn = Authn::from_methods(&ctx.methods, ctx.auth_time);
        let issued = finish_and_issue(token, ctx, &authn);
        if issued.status != 200 {
            return issued;
        }
        body["tokens"] = issued.body;
    }
    HttpJson::ok(body)
}

/// The security page's view of an account (`MfaFactorsResponse`): confirmed factors, recovery codes left, policy.
#[must_use]
pub fn factors_response(ctx: &MfaContext) -> HttpJson<Value> {
    let listed = factors::list_active(&ctx.tenant_id, ctx.user_id).and_then(|active| {
        let left = if active.is_empty() {
            0
        } else {
            factors::recovery_codes_left(&ctx.tenant_id, ctx.user_id)?
        };
        let required = policy::required_for(&ctx.tenant_id, ctx.user_id)?;
        Ok((active, left, required))
    });
    match listed {
        Ok((active, left, required)) => HttpJson::ok(json!({
            "factors": active.iter().map(factor_json).collect::<Vec<_>>(),
            "recovery_codes_left": left,
            "required": required.as_str(),
            "totp_available": totp::available(),
            "passkeys_available": passkey::available(),
        })),
        Err(e) => {
            tracing::error!(error = %e, "mfa: factor listing failed");
            unavailable()
        }
    }
}

// ── shared error bodies ─────────────────────────────────────────────────────

#[must_use]
pub fn invalid_token() -> HttpJson<Value> {
    HttpJson::new(
        401,
        json!({"error": "invalid_token", "error_description": "This sign-in step has expired or was already used. Sign in again."}),
    )
}

#[must_use]
pub fn invalid_code() -> HttpJson<Value> {
    HttpJson::new(
        401,
        json!({"error": "invalid_code", "error_description": "That code is not valid. Check it and try again."}),
    )
}

#[must_use]
pub fn unavailable() -> HttpJson<Value> {
    HttpJson::new(
        503,
        json!({"error": "temporarily_unavailable", "error_description": "Sign-in is briefly unavailable. Try again in a moment."}),
    )
}

#[must_use]
pub fn error(status: u16, code: &str, description: &str) -> HttpJson<Value> {
    HttpJson::new(
        status,
        json!({"error": code, "error_description": description}),
    )
}

/// The abuse-guard identity for second-factor guessing on one account: shared by every second-factor endpoint, so
/// guessing TOTP codes and recovery codes locks the same counter.
#[must_use]
pub fn guard_identity(user_id: &Uuid) -> String {
    format!("mfa:{user_id}")
}

#[cfg(test)]
mod tests {
    use super::*;

    const PK: Method = Method::Passkey {
        user_verified: true,
        synced: true,
    };

    fn held(totp: bool, passkey: bool, codes: bool) -> Holdings {
        Holdings {
            totp,
            passkey,
            recovery_codes: codes,
        }
    }

    #[test]
    fn no_factor_and_no_policy_signs_in_with_a_password() {
        match decide(
            &[Method::Password],
            1,
            Holdings::default(),
            Requirement::None,
        ) {
            Next::Issue(a) => {
                assert_eq!(a.amr, ["pwd"]);
                assert_eq!(a.auth_time, 1);
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn an_account_with_a_factor_is_always_challenged_after_a_first_factor() {
        for first in [
            Method::Password,
            Method::Email,
            Method::Federated,
            Method::Sms,
        ] {
            assert_eq!(
                decide(&[first], 1, held(true, false, true), Requirement::None),
                Next::Challenge(vec!["totp", "recovery_code"]),
                "{first:?}"
            );
        }
        assert_eq!(
            decide(
                &[Method::Password],
                1,
                held(true, true, false),
                Requirement::None
            ),
            Next::Challenge(vec!["totp", "passkey"])
        );
    }

    #[test]
    fn a_second_factor_completes_the_sign_in_truthfully() {
        match decide(
            &[Method::Password, Method::Totp],
            1,
            held(true, false, true),
            Requirement::Mfa,
        ) {
            Next::Issue(a) => assert_eq!(a.amr, ["mfa", "otp", "pwd"]),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn a_passkey_alone_satisfies_every_policy() {
        for required in [Requirement::None, Requirement::Mfa, Requirement::Passkey] {
            assert!(matches!(
                decide(&[PK], 1, held(true, true, true), required),
                Next::Issue(_)
            ));
        }
    }

    #[test]
    fn policy_forces_enrolment_for_a_member_without_a_factor() {
        assert_eq!(
            decide(
                &[Method::Password],
                1,
                Holdings::default(),
                Requirement::Mfa
            ),
            Next::Enroll(Requirement::Mfa)
        );
        assert_eq!(
            decide(
                &[Method::Password],
                1,
                Holdings::default(),
                Requirement::Passkey
            ),
            Next::Enroll(Requirement::Passkey)
        );
    }

    #[test]
    fn passkey_policy_offers_only_the_passkey_and_recovery() {
        assert_eq!(
            decide(
                &[Method::Password],
                1,
                held(true, true, true),
                Requirement::Passkey
            ),
            Next::Challenge(vec!["passkey", "recovery_code"])
        );
        // TOTP-only member of a passkey org: verify the TOTP, then enrol a passkey.
        assert_eq!(
            decide(
                &[Method::Password],
                1,
                held(true, false, false),
                Requirement::Passkey
            ),
            Next::Challenge(vec!["totp"])
        );
        assert_eq!(
            decide(
                &[Method::Password, Method::Totp],
                1,
                held(true, false, false),
                Requirement::Passkey
            ),
            Next::Enroll(Requirement::Passkey)
        );
    }

    /// ADR-008 §6: a recovery code never stands in for a required passkey.
    #[test]
    fn recovery_under_a_passkey_policy_requires_a_new_passkey() {
        assert_eq!(
            decide(
                &[Method::Password, Method::RecoveryCode],
                1,
                held(false, true, true),
                Requirement::Passkey
            ),
            Next::Enroll(Requirement::Passkey)
        );
        // Without a passkey policy, recovery signs in at aal2.
        match decide(
            &[Method::Password, Method::RecoveryCode],
            1,
            held(true, false, true),
            Requirement::Mfa,
        ) {
            Next::Issue(a) => assert_eq!(a.acr, sesame_common::authn::ACR_AAL2),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn an_email_code_is_never_the_second_factor() {
        assert_eq!(
            decide(
                &[Method::Password, Method::Email],
                1,
                held(true, false, false),
                Requirement::None
            ),
            Next::Challenge(vec!["totp"])
        );
    }
}
