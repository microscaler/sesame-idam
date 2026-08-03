//! Credential field redaction for structured logs (Epic 14.8).
//!
//! Field names match `conformance/oidc-v1/manifest.json` `redacted_fields`.

use serde_json::Value;

/// Default redacted field names (kept in sync with the OIDC conformance manifest).
pub const DEFAULT_REDACTED_FIELDS: &[&str] = &[
    "access_token",
    "refresh_token",
    "id_token",
    "code",
    "code_verifier",
    "client_secret",
];

/// Return the redacted field list from a manifest JSON document, or defaults.
#[must_use]
pub fn redacted_field_names(manifest: Option<&Value>) -> Vec<String> {
    manifest
        .and_then(|m| m.get("redacted_fields"))
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect()
        })
        .filter(|v: &Vec<String>| !v.is_empty())
        .unwrap_or_else(|| {
            DEFAULT_REDACTED_FIELDS
                .iter()
                .map(|s| (*s).to_string())
                .collect()
        })
}

/// Recursively replace values of redacted keys with `"***"`.
pub fn redact_sensitive_object(value: &mut Value, fields: &[String]) {
    match value {
        Value::Object(map) => {
            let keys: Vec<String> = map.keys().cloned().collect();
            for key in keys {
                if fields.iter().any(|f| f == &key) {
                    map.insert(key, Value::String("***".into()));
                } else if let Some(child) = map.get_mut(&key) {
                    redact_sensitive_object(child, fields);
                }
            }
        }
        Value::Array(items) => {
            for item in items {
                redact_sensitive_object(item, fields);
            }
        }
        _ => {}
    }
}

/// Return `Err` with the first field name found as a non-redacted string value.
pub fn assert_no_redacted_fields(sample: &Value, fields: &[String]) -> Result<(), String> {
    fn walk(value: &Value, fields: &[String], path: &str) -> Result<(), String> {
        match value {
            Value::Object(map) => {
                for (key, child) in map {
                    let child_path = if path.is_empty() {
                        key.clone()
                    } else {
                        format!("{path}.{key}")
                    };
                    if fields.iter().any(|f| f == key) {
                        match child {
                            Value::String(s) if s != "***" && !s.is_empty() => {
                                return Err(format!(
                                    "redacted field `{child_path}` present with value length {}",
                                    s.len()
                                ));
                            }
                            Value::Null | Value::String(_) => {}
                            other => {
                                return Err(format!(
                                    "redacted field `{child_path}` has non-string value: {other}"
                                ));
                            }
                        }
                    } else {
                        walk(child, fields, &child_path)?;
                    }
                }
                Ok(())
            }
            Value::Array(items) => {
                for (i, item) in items.iter().enumerate() {
                    walk(item, fields, &format!("{path}[{i}]"))?;
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }
    walk(sample, fields, "")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_nested_tokens() {
        let mut sample = serde_json::json!({
            "event": "oauth_token",
            "body": {
                "access_token": "eyJhbGciOiJFZERTQSJ9.aaa.bbb",
                "refresh_token": "refresh.secret",
                "scope": "openid"
            }
        });
        let fields = redacted_field_names(None);
        redact_sensitive_object(&mut sample, &fields);
        assert_eq!(sample["body"]["access_token"], "***");
        assert_eq!(sample["body"]["refresh_token"], "***");
        assert_eq!(sample["body"]["scope"], "openid");
        assert!(assert_no_redacted_fields(&sample, &fields).is_ok());
    }

    #[test]
    fn detect_leaked_code_verifier() {
        let sample = serde_json::json!({
            "code_verifier": "abcdefghijklmnopqrstuvwxyz"
        });
        let fields = redacted_field_names(None);
        let err = assert_no_redacted_fields(&sample, &fields).unwrap_err();
        assert!(err.contains("code_verifier"));
    }
}
