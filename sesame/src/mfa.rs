#[cfg(test)]
mod tests {
    use crate::test_utils::TestEnv;
    use serde_json::json;


    #[tokio::test]
    #[ignore] // ignore failed test
    async fn test_verify_totp_challenge() {
        let env = TestEnv::new("/api/backend/v1/mfa/step-up/verify-totp").await;

        let payload = json!({
            "action_type": "SENSITIVE_ACTION",
            "user_id": "a04d69d7-9347-48a3-aa01-8e7ce9aeee04",
            "code": "123456",
            "grant_type": "TIME_BASED",
            "valid_for_seconds": 60
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
                "step_up_grant": "765ba468d62c61f30b17098ca6bf9d5e755fbd79caf012c3ffed79a1edf602ad9d63e7f8166b6a4b71c8059ef0da2db4"
            }),
            "Unexpected response JSON"
        );
    }

    #[tokio::test]
    #[ignore] // ignore failed test
    async fn test_verify_step_up_grant() {
        let env = TestEnv::new("/api/backend/v1/mfa/step-up/verify-grant").await;

        let payload = json!({
            "action_type": "SENSITIVE_ACTION",
            "user_id": "a04d69d7-9347-48a3-aa01-8e7ce9aeee04",
            "grant": "765ba468d62..."
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
}
