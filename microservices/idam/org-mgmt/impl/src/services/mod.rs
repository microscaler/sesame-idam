pub mod org_lifecycle;
pub mod owner_transfer_otp;
pub mod password_verify;

/// Invite magic-link base URL (product onboarding). Env: `INVITE_MAGIC_LINK_BASE`.
pub fn invite_magic_link_url(token: &str) -> String {
    let base = std::env::var("INVITE_MAGIC_LINK_BASE").unwrap_or_else(|_| {
        "https://loadlinker.dev.microscaler.local/onboarding".to_string()
    });
    let sep = if base.contains('?') { '&' } else { '?' };
    format!("{base}{sep}token={token}")
}
