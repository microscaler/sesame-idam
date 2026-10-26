pub mod auth_policy;
pub mod domain_api;
pub mod domain_claims;
pub mod org_lifecycle;
pub mod owner_transfer_otp;
pub mod password_verify;

/// Invite magic-link URL (product onboarding). Env: `INVITE_MAGIC_LINK_BASE`, the onboarding page of the product the
/// tenant runs. Unset: a development default with `SESAME_DEV_MODE=1`, otherwise `None` (no link can be sent).
#[must_use]
pub fn invite_magic_link_url(token: &str) -> Option<String> {
    let base = match std::env::var("INVITE_MAGIC_LINK_BASE") {
        Ok(v) if !v.trim().is_empty() => v.trim().to_string(),
        _ if sesame_common::deployment::dev_mode() => {
            "https://loadlinker.dev.microscaler.local/onboarding".to_string()
        }
        _ => return None,
    };
    let sep = if base.contains('?') { '&' } else { '?' };
    Some(format!("{base}{sep}token={token}"))
}
