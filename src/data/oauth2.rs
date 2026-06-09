use base64::{engine::general_purpose, Engine as _};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OAuth2TokenResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: Option<u64>,
    pub refresh_token: Option<String>,
    pub scope: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DeviceAuthorizationResponse {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub verification_uri_complete: Option<String>,
    pub expires_in: u64,
    pub interval: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DeviceTokenResponse {
    pub access_token: Option<String>,
    pub token_type: Option<String>,
    pub expires_in: Option<u64>,
    pub refresh_token: Option<String>,
    pub scope: Option<String>,
    pub error: Option<String>,
    pub error_description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OAuth2ErrorResponse {
    pub error: String,
    pub error_description: Option<String>,
}

#[derive(Debug, Clone)]
pub struct PKCEChallenge {
    pub verifier: String,
    pub challenge: String,
}

impl PKCEChallenge {
    pub fn generate() -> Self {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let verifier: String = (0..128)
            .map(|_| {
                let idx = rng.gen_range(0..62);
                match idx {
                    0..=9 => (b'0' + idx) as char,
                    10..=35 => (b'a' + (idx - 10)) as char,
                    36..=61 => (b'A' + (idx - 36)) as char,
                    _ => unreachable!(),
                }
            })
            .collect();

        let digest = Sha256::digest(verifier.as_bytes());
        let challenge = general_purpose::URL_SAFE_NO_PAD.encode(digest);

        Self { verifier, challenge }
    }
}

pub fn build_authorization_url(
    auth_url: &str,
    client_id: &str,
    redirect_uri: &str,
    scopes: &str,
    pkce: Option<&PKCEChallenge>,
    state: &str,
) -> String {
    let mut url = format!(
        "{}?response_type=code&client_id={}&redirect_uri={}&state={}",
        auth_url,
        urlencoding::encode(client_id),
        urlencoding::encode(redirect_uri),
        urlencoding::encode(state),
    );

    if !scopes.is_empty() {
        url.push_str(&format!("&scope={}", urlencoding::encode(scopes)));
    }

    if let Some(pkce) = pkce {
        url.push_str(&format!(
            "&code_challenge={}&code_challenge_method=S256",
            urlencoding::encode(&pkce.challenge)
        ));
    }

    url
}

pub async fn exchange_code(
    token_url: &str,
    code: &str,
    client_id: &str,
    client_secret: &str,
    redirect_uri: &str,
    pkce_verifier: Option<&str>,
) -> Result<OAuth2TokenResponse, String> {
    let client = Client::new();
    let mut params = vec![
        ("grant_type", "authorization_code"),
        ("code", code),
        ("client_id", client_id),
        ("redirect_uri", redirect_uri),
    ];

    if !client_secret.is_empty() {
        params.push(("client_secret", client_secret));
    }

    if let Some(verifier) = pkce_verifier {
        params.push(("code_verifier", verifier));
    }

    let response = client
        .post(token_url)
        .form(&params)
        .send()
        .await
        .map_err(|e| format!("Failed to send token request: {}", e))?;

    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|e| format!("Failed to read response body: {}", e))?;

    if status.is_success() {
        serde_json::from_str(&body)
            .map_err(|e| format!("Failed to parse token response: {}", e))
    } else {
        let error: OAuth2ErrorResponse =
            serde_json::from_str(&body).unwrap_or_else(|_| OAuth2ErrorResponse {
                error: "unknown_error".to_string(),
                error_description: Some(body),
            });
        Err(format!(
            "Token request failed: {} - {}",
            error.error,
            error.error_description.unwrap_or_default()
        ))
    }
}

pub async fn client_credentials(
    token_url: &str,
    client_id: &str,
    client_secret: &str,
    scopes: &str,
) -> Result<OAuth2TokenResponse, String> {
    let client = Client::new();
    let mut params = vec![("grant_type", "client_credentials")];

    if !client_id.is_empty() {
        params.push(("client_id", client_id));
    }

    if !client_secret.is_empty() {
        params.push(("client_secret", client_secret));
    }

    if !scopes.is_empty() {
        params.push(("scope", scopes));
    }

    let response = client
        .post(token_url)
        .form(&params)
        .send()
        .await
        .map_err(|e| format!("Failed to send client credentials request: {}", e))?;

    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|e| format!("Failed to read response body: {}", e))?;

    if status.is_success() {
        serde_json::from_str(&body)
            .map_err(|e| format!("Failed to parse token response: {}", e))
    } else {
        let error: OAuth2ErrorResponse =
            serde_json::from_str(&body).unwrap_or_else(|_| OAuth2ErrorResponse {
                error: "unknown_error".to_string(),
                error_description: Some(body),
            });
        Err(format!(
            "Client credentials request failed: {} - {}",
            error.error,
            error.error_description.unwrap_or_default()
        ))
    }
}

pub async fn refresh_token(
    token_url: &str,
    refresh_token: &str,
    client_id: &str,
    client_secret: &str,
) -> Result<OAuth2TokenResponse, String> {
    let client = Client::new();
    let mut params = vec![
        ("grant_type", "refresh_token"),
        ("refresh_token", refresh_token),
        ("client_id", client_id),
    ];

    if !client_secret.is_empty() {
        params.push(("client_secret", client_secret));
    }

    let response = client
        .post(token_url)
        .form(&params)
        .send()
        .await
        .map_err(|e| format!("Failed to send refresh token request: {}", e))?;

    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|e| format!("Failed to read response body: {}", e))?;

    if status.is_success() {
        serde_json::from_str(&body)
            .map_err(|e| format!("Failed to parse token response: {}", e))
    } else {
        let error: OAuth2ErrorResponse =
            serde_json::from_str(&body).unwrap_or_else(|_| OAuth2ErrorResponse {
                error: "unknown_error".to_string(),
                error_description: Some(body),
            });
        Err(format!(
            "Token refresh failed: {} - {}",
            error.error,
            error.error_description.unwrap_or_default()
        ))
    }
}

pub fn generate_state() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    (0..32)
        .map(|_| {
            let idx = rng.gen_range(0..16);
            format!("{:x}", idx)
        })
        .collect()
}

pub async fn device_authorization(
    device_auth_url: &str,
    client_id: &str,
    scopes: &str,
) -> Result<DeviceAuthorizationResponse, String> {
    let client = Client::new();
    let mut params = vec![("client_id", client_id)];

    if !scopes.is_empty() {
        params.push(("scope", scopes));
    }

    let response = client
        .post(device_auth_url)
        .form(&params)
        .send()
        .await
        .map_err(|e| format!("Failed to send device authorization request: {}", e))?;

    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|e| format!("Failed to read response body: {}", e))?;

    if status.is_success() {
        serde_json::from_str(&body)
            .map_err(|e| format!("Failed to parse device authorization response: {}", e))
    } else {
        let error: OAuth2ErrorResponse =
            serde_json::from_str(&body).unwrap_or_else(|_| OAuth2ErrorResponse {
                error: "unknown_error".to_string(),
                error_description: Some(body),
            });
        Err(format!(
            "Device authorization failed: {} - {}",
            error.error,
            error.error_description.unwrap_or_default()
        ))
    }
}

pub async fn poll_device_token(
    token_url: &str,
    device_code: &str,
    client_id: &str,
    client_secret: &str,
) -> Result<DeviceTokenResponse, String> {
    let client = Client::new();
    let mut params = vec![
        ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
        ("device_code", device_code),
        ("client_id", client_id),
    ];

    if !client_secret.is_empty() {
        params.push(("client_secret", client_secret));
    }

    let response = client
        .post(token_url)
        .form(&params)
        .send()
        .await
        .map_err(|e| format!("Failed to poll device token: {}", e))?;

    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|e| format!("Failed to read response body: {}", e))?;

    if status.is_success() {
        serde_json::from_str(&body)
            .map_err(|e| format!("Failed to parse device token response: {}", e))
    } else {
        let error: OAuth2ErrorResponse =
            serde_json::from_str(&body).unwrap_or_else(|_| OAuth2ErrorResponse {
                error: "unknown_error".to_string(),
                error_description: Some(body),
            });
        Err(format!(
            "Device token poll failed: {} - {}",
            error.error,
            error.error_description.unwrap_or_default()
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pkce_challenge_generation() {
        let pkce = PKCEChallenge::generate();
        assert_eq!(pkce.verifier.len(), 128);
        assert!(!pkce.challenge.is_empty());
    }

    #[test]
    fn pkce_challenge_deterministic() {
        let pkce1 = PKCEChallenge::generate();
        let digest = Sha256::digest(pkce1.verifier.as_bytes());
        let expected_challenge = general_purpose::URL_SAFE_NO_PAD.encode(digest);
        assert_eq!(pkce1.challenge, expected_challenge);
    }

    #[test]
    fn build_authorization_url_basic() {
        let url = build_authorization_url(
            "https://auth.example.com/authorize",
            "client123",
            "http://localhost:8080/callback",
            "",
            None,
            "state123",
        );
        assert!(url.starts_with("https://auth.example.com/authorize?"));
        assert!(url.contains("response_type=code"));
        assert!(url.contains("client_id=client123"));
        assert!(url.contains("redirect_uri=http%3A%2F%2Flocalhost%3A8080%2Fcallback"));
        assert!(url.contains("state=state123"));
    }

    #[test]
    fn build_authorization_url_with_scopes() {
        let url = build_authorization_url(
            "https://auth.example.com/authorize",
            "client123",
            "http://localhost:8080/callback",
            "read write",
            None,
            "state123",
        );
        assert!(url.contains("scope=read%20write"));
    }

    #[test]
    fn build_authorization_url_with_pkce() {
        let pkce = PKCEChallenge::generate();
        let url = build_authorization_url(
            "https://auth.example.com/authorize",
            "client123",
            "http://localhost:8080/callback",
            "",
            Some(&pkce),
            "state123",
        );
        assert!(url.contains("code_challenge="));
        assert!(url.contains("code_challenge_method=S256"));
    }

    #[test]
    fn generate_state_returns_32_hex_chars() {
        let state = generate_state();
        assert_eq!(state.len(), 32);
        assert!(state.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn token_response_deserialization() {
        let json = r#"{
            "access_token": "test_token",
            "token_type": "Bearer",
            "expires_in": 3600,
            "refresh_token": "refresh_token",
            "scope": "read write"
        }"#;
        let response: OAuth2TokenResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.access_token, "test_token");
        assert_eq!(response.token_type, "Bearer");
        assert_eq!(response.expires_in, Some(3600));
        assert_eq!(response.refresh_token, Some("refresh_token".to_string()));
        assert_eq!(response.scope, Some("read write".to_string()));
    }

    #[test]
    fn error_response_deserialization() {
        let json = r#"{
            "error": "invalid_grant",
            "error_description": "Authorization code expired"
        }"#;
        let response: OAuth2ErrorResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.error, "invalid_grant");
        assert_eq!(
            response.error_description,
            Some("Authorization code expired".to_string())
        );
    }

    #[test]
    fn device_authorization_response_deserialization() {
        let json = r#"{
            "device_code": "device_code_123",
            "user_code": "ABCD-1234",
            "verification_uri": "https://auth.example.com/device",
            "verification_uri_complete": "https://auth.example.com/device?user_code=ABCD-1234",
            "expires_in": 600,
            "interval": 5
        }"#;
        let response: DeviceAuthorizationResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.device_code, "device_code_123");
        assert_eq!(response.user_code, "ABCD-1234");
        assert_eq!(response.verification_uri, "https://auth.example.com/device");
        assert_eq!(response.expires_in, 600);
        assert_eq!(response.interval, Some(5));
    }

    #[test]
    fn device_token_response_pending_deserialization() {
        let json = r#"{
            "error": "authorization_pending",
            "error_description": "The authorization request is still pending"
        }"#;
        let response: DeviceTokenResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.error, Some("authorization_pending".to_string()));
        assert!(response.access_token.is_none());
    }

    #[test]
    fn device_token_response_success_deserialization() {
        let json = r#"{
            "access_token": "device_token_123",
            "token_type": "Bearer",
            "expires_in": 3600,
            "refresh_token": "refresh_device_token"
        }"#;
        let response: DeviceTokenResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.access_token, Some("device_token_123".to_string()));
        assert_eq!(response.token_type, Some("Bearer".to_string()));
        assert_eq!(response.expires_in, Some(3600));
    }
}
