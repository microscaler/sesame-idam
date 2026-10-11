//! Gate A6 drift guard: Helm BearerAuth.iss must match the development SESAME_JWT_ISSUER, and the chart itself
//! must not default the issuer.
//!
//! Login mints tokens with `SESAME_JWT_ISSUER`; BRRTRouter validates Bearer
//! tokens against `security.jwks.BearerAuth.iss`. These two must agree or every
//! Bearer-protected route returns `401 invalid_token`.

use std::fs;
use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("resolve sesame-idam repo root from idam/common")
}

fn yaml_string_value(doc: &serde_yaml::Value, path: &[&str]) -> Option<String> {
    let mut cur = doc;
    for key in path {
        cur = cur.get(*key)?;
    }
    cur.as_str().map(|s| s.to_string())
}

#[test]
fn chart_sesame_jwt_issuer_matches_service_bearer_iss() {
    let root = repo_root();
    let read = |rel: &str| -> serde_yaml::Value {
        serde_yaml::from_str(
            &fs::read_to_string(root.join(rel)).unwrap_or_else(|e| panic!("read {rel}: {e}")),
        )
        .unwrap_or_else(|e| panic!("parse {rel}: {e}"))
    };

    // The chart has no issuer and no development mode of its own: a deployment names its issuer.
    let chart = read("helm/sesame-idam-microservice/values.yaml");
    assert_eq!(
        yaml_string_value(&chart, &["env", "SESAME_JWT_ISSUER"]).as_deref(),
        Some(""),
        "chart values.yaml must not default the issuer"
    );
    assert_eq!(
        chart.get("devMode").and_then(serde_yaml::Value::as_bool),
        Some(false),
        "chart values.yaml must default devMode to false"
    );

    // Development (Tilt) sets the issuer in values/_dev.yaml.
    let dev = read("helm/sesame-idam-microservice/values/_dev.yaml");
    let issuer = yaml_string_value(&dev, &["env", "SESAME_JWT_ISSUER"])
        .expect("env.SESAME_JWT_ISSUER must be set in values/_dev.yaml");

    let service_files = [
        "identity-login-service.yaml",
        "identity-session-service.yaml",
        "org-mgmt.yaml",
        "identity-user-mgmt-service.yaml",
        "authz-core.yaml",
        "api-keys.yaml",
    ];

    for name in service_files {
        let path = root.join("helm/sesame-idam-microservice/values").join(name);
        let doc: serde_yaml::Value = serde_yaml::from_str(
            &fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display())),
        )
        .unwrap_or_else(|e| panic!("parse {}: {e}", path.display()));

        let iss = yaml_string_value(
            &doc,
            &["app", "config", "security", "jwks", "BearerAuth", "iss"],
        )
        .unwrap_or_else(|| panic!("{} missing app.config.security.jwks.BearerAuth.iss", name));

        assert_eq!(
            iss, issuer,
            "{name}: BearerAuth.iss ({iss}) must equal the development SESAME_JWT_ISSUER ({issuer})"
        );
    }

    let flux_common = root
        .join("deployment-configuration/profiles/dev/sesame-idam/idam/services/values/common.yaml");
    let common: serde_yaml::Value =
        serde_yaml::from_str(&fs::read_to_string(&flux_common).expect("read flux common values"))
            .expect("parse flux common values");
    let common_iss = yaml_string_value(
        &common,
        &["app", "config", "security", "jwks", "BearerAuth", "iss"],
    )
    .expect("flux common BearerAuth.iss");
    assert_eq!(
        common_iss, issuer,
        "flux common BearerAuth.iss must equal the development SESAME_JWT_ISSUER"
    );
    assert_eq!(
        yaml_string_value(&common, &["env", "SESAME_JWT_ISSUER"]).as_deref(),
        Some(issuer.as_str()),
        "flux common env.SESAME_JWT_ISSUER must equal its BearerAuth.iss"
    );
}
