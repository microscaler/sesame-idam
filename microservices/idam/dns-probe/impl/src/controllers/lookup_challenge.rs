// BRRTRouter: user-owned
//! `POST /lookup`: every configured provider's answer for one challenge name.

use std::sync::LazyLock;

use brrtrouter::typed::{HttpJson, TypedHandlerRequest};
use brrtrouter_macros::handler;
use sesame_common::domain_verify::doh::{
    checked_challenge_name, parse_providers, query_all, Provider, ProviderConfigError,
    DEFAULT_PROVIDERS, PROVIDERS_ENV, PROVIDER_TIMEOUT,
};
use sesame_idam_dns_probe_gen::handlers::lookup_challenge::Request;

static PROVIDERS: LazyLock<Result<Vec<Provider>, ProviderConfigError>> = LazyLock::new(|| {
    let spec = std::env::var(PROVIDERS_ENV).unwrap_or_else(|_| DEFAULT_PROVIDERS.to_string());
    parse_providers(&spec)
});

/// The configured providers (checked at start-up by `main`).
///
/// # Errors
///
/// [`ProviderConfigError`] for a list without two distinct operators.
pub fn providers() -> Result<&'static [Provider], ProviderConfigError> {
    PROVIDERS.as_deref().map_err(Clone::clone)
}

#[handler(LookupChallengeController)]
#[allow(clippy::needless_pass_by_value)] // the handler signature is fixed by the macro
pub fn handle(req: TypedHandlerRequest<Request>) -> HttpJson<serde_json::Value> {
    let Some(name) = checked_challenge_name(&req.data.name) else {
        return HttpJson::new(422, serde_json::json!({ "reason": "not_a_challenge_name" }));
    };
    let Ok(providers) = providers() else {
        return HttpJson::new(503, serde_json::json!({ "reason": "no_providers" }));
    };
    let answers = query_all(providers, &name, PROVIDER_TIMEOUT);
    for a in &answers {
        // Name and outcome only; record contents stay out of the log.
        let outcome = match &a.outcome {
            sesame_common::domain_verify::ProviderOutcome::Answered(d) => {
                format!("rcode={} ad={}", d.rcode, d.ad)
            }
            sesame_common::domain_verify::ProviderOutcome::Failed { error } => {
                format!("failed={error}")
            }
        };
        tracing::info!(%name, provider = %a.provider, %outcome, "doh lookup");
    }
    HttpJson::ok(serde_json::json!({ "name": name, "answers": answers }))
}
