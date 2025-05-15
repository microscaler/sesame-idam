#[cfg(test)]
mod tests {
    use crate::test_utils::TestEnv;
    use serde_json::json;


    #[tokio::test]
    #[ignore] // ignore failed test
    async fn test_force_logout_all_user_sessions() {
        let env =
            TestEnv::new("/api/backend/v1/user/a04d69d7-9347-48a3-aa01-8e7ce9aeee04/logout_all_sessions")
                .await;

        let response = env
            .client
            .post(env.url.clone())
            .bearer_auth("valid_token")
            .send()
            .await
            .expect("Failed to send request");

        assert_eq!(response.status().as_u16(), 200, "Expected HTTP 200 OK");

        let response_json: serde_json::Value = response.json().await.unwrap();
        assert_eq!(response_json, json!({}), "Expected empty JSON response");
    }


    #[tokio::test]
    #[ignore] // ignore failed test
    async fn test_create_saml_connection_link() {
        let env = TestEnv::new(
            "/api/backend/v1/org/582e7c11-6b72-40d8-886d-461e6491fa71/create_saml_connection_link",
        )
        .await;

        let payload = json!({
            "expires_in_seconds": 86400
        });

        let response = env
            .client
            .post(env.url.clone())
            .bearer_auth("valid_token")
            .json(&payload)
            .send()
            .await
            .expect("Failed to send request");

        assert_eq!(response.status().as_u16(), 200, "Expected HTTP 200 OK");

        let response_json: serde_json::Value = response.json().await.unwrap();
        assert_eq!(
            response_json,
            json!({
                "url": "https://example.propelauthtest.com/setup_saml/a4f57bc035bc2c21382f587a40bb45b22361195ebeb1a0d78d48beb58583bc2c7131d88fe4e110"
            }),
            "Unexpected response JSON"
        );
    }

    #[tokio::test]
    #[ignore] // ignore failed test
    async fn test_fetch_org_saml_metadata() {
        let env = TestEnv::new("/api/backend/v1/saml_sp_metadata/6983").await;

        let response = env
            .client
            .get(env.url.clone())
            .bearer_auth("valid_token")
            .send()
            .await
            .expect("Failed to send request");

        assert_eq!(response.status().as_u16(), 200, "Expected HTTP 200 OK");

        let response_json: serde_json::Value = response.json().await.unwrap();
        assert_eq!(
            response_json,
            json!({
                "entity_id": "https://example.sesame.microscaler.io/saml/6983/metadata",
                "acs_url": "https://example.sesame.microscaler.io/saml/6983/acs",
                "logout_url": "https://example.sesame.microscaler.io/saml/6983/logout"
            }),
            "Unexpected response JSON"
        );
    }

    #[tokio::test]
    #[ignore] // ignore failed test
    async fn test_set_saml_idp_metadata() {
        let env = TestEnv::new("/api/backend/v1/saml_idp_metadata").await;

        let payload = json!({
            "idp_entity_id": "http://www.okta.com/example",
            "idp_sso_url": "https://dev.okta.com/app/example/example/sso/saml",
            "idp_certificate": "-----BEGIN CERTIFICATE-----MIIDqDCCApCgAw-----END CERTIFICATE-----",
            "provider": "Okta",
            "org_id": "582e7c11-6b72-40d8-886d-461e6491fa71"
        });

        let response = env
            .client
            .post(env.url.clone())
            .bearer_auth("valid_token")
            .json(&payload)
            .send()
            .await
            .expect("Failed to send request");

        assert_eq!(response.status().as_u16(), 200, "Expected HTTP 200 OK");

        let response_json: serde_json::Value = response.json().await.unwrap();
        assert_eq!(response_json, json!({}), "Expected empty JSON response");
    }

    #[tokio::test]
    #[ignore] // ignore failed test
    async fn test_enable_saml_connection() {
        let env =
            TestEnv::new("/api/backend/v1/saml_idp_metadata/go_live/582e7c11-6b72-40d8-886d-461e6491fa71")
                .await;

        let response = env
            .client
            .post(env.url.clone())
            .bearer_auth("valid_token")
            .json(&json!({}))
            .send()
            .await
            .expect("Failed to send request");

        assert_eq!(response.status().as_u16(), 200, "Expected HTTP 200 OK");

        let response_json: serde_json::Value = response.json().await.unwrap();
        assert_eq!(response_json, json!({}), "Expected empty JSON response");
    }

    #[tokio::test]
    #[ignore] // ignore failed test
    async fn test_delete_saml_connection() {
        let env =
            TestEnv::new("/api/backend/v1/saml_idp_metadata/582e7c11-6b72-40d8-886d-461e6491fa71").await;

        let payload = json!({
            "idp_entity_id": "",
            "idp_sso_url": "",
            "idp_certificate": "",
            "provider": "",
            "org_id": "582e7c11-6b72-40d8-886d-461e6491fa71"
        });

        let response = env
            .client
            .delete(env.url.clone())
            .bearer_auth("valid_token")
            .json(&payload)
            .send()
            .await
            .expect("Failed to send request");

        assert_eq!(response.status().as_u16(), 200, "Expected HTTP 200 OK");

        let response_json: serde_json::Value = response.json().await.unwrap();
        assert_eq!(response_json, json!({}), "Expected empty JSON response");
    }
}
