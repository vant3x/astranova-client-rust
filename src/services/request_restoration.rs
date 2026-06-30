use crate::http_client::request::HttpRequest;
use crate::persistence::database::{CollectionRequest, RequestHistoryEntry};
use crate::ui::components::key_value_editor::KeyValueEntry;
use crate::ui::views::http_request_view::{BodyType, HttpRequestView};

pub fn build_view_from_history(entry: &RequestHistoryEntry) -> Option<HttpRequestView> {
    let mut view = HttpRequestView::default();

    if let Some(request_data) = &entry.request_data {
        if let Ok(request) = serde_json::from_str::<HttpRequest>(request_data) {
            apply_request_to_view(&mut view, &request);
        } else {
            view.url_input = entry.url.clone();
            view.method = entry.method.clone();
        }
    } else {
        view.url_input = entry.url.clone();
        view.method = entry.method.clone();
    }

    Some(view)
}

pub fn build_view_from_collection_request(req: &CollectionRequest) -> HttpRequestView {
    let mut view = HttpRequestView::default();
    view.url_input = req.url.clone();
    view.method = req.method.clone();

    if let Some(body) = &req.body {
        view.body_input = iced::widget::text_editor::Content::with_text(body);
    }

    if req.body_type == "multipart" {
        view.body_type = BodyType::Multipart;
    }

    view.headers_editor.entries = req
        .headers
        .iter()
        .enumerate()
        .map(|(i, (k, v))| KeyValueEntry {
            id: i,
            key: k.clone(),
            value: v.clone(),
        })
        .collect();

    view.params_editor.entries = req
        .params
        .iter()
        .enumerate()
        .map(|(i, (k, v))| KeyValueEntry {
            id: i,
            key: k.clone(),
            value: v.clone(),
        })
        .collect();

    if let Some(data) = &req.auth_data {
        if data.starts_with('{') {
            if let Ok(auth) = serde_json::from_str::<crate::data::auth::Auth>(data) {
                view.auth = auth;
            }
        } else {
            match req.auth_type.as_str() {
                "bearer" => {
                    view.auth = crate::data::auth::Auth::BearerToken(data.clone());
                }
                "basic" => {
                    let parts: Vec<&str> = data.splitn(2, ':').collect();
                    if parts.len() == 2 {
                        view.auth = crate::data::auth::Auth::Basic {
                            user: parts[0].to_string(),
                            pass: parts[1].to_string(),
                        };
                    }
                }
                "api_key" => {
                    let parts: Vec<&str> = data.splitn(3, ':').collect();
                    if parts.len() == 3 {
                        let location = match parts[2] {
                            "query" => crate::data::auth::ApiKeyLocation::Query,
                            _ => crate::data::auth::ApiKeyLocation::Header,
                        };
                        view.auth = crate::data::auth::Auth::ApiKey {
                            key: parts[0].to_string(),
                            value: parts[1].to_string(),
                            location,
                        };
                    } else if parts.len() == 2 {
                        view.auth = crate::data::auth::Auth::ApiKey {
                            key: parts[0].to_string(),
                            value: parts[1].to_string(),
                            location: crate::data::auth::ApiKeyLocation::Header,
                        };
                    }
                }
                "digest" => {
                    let parts: Vec<&str> = data.splitn(2, ':').collect();
                    if parts.len() == 2 {
                        view.auth = crate::data::auth::Auth::Digest {
                            user: parts[0].to_string(),
                            pass: parts[1].to_string(),
                        };
                    }
                }
                "oauth2" => {
                    view.auth = crate::data::auth::Auth::OAuth2(Box::new(
                        crate::data::auth::OAuth2Config {
                            access_token: data.clone(),
                            ..Default::default()
                        },
                    ));
                }
                _ => {}
            }
        }
    }

    view
}

fn apply_request_to_view(view: &mut HttpRequestView, request: &HttpRequest) {
    view.url_input = request.url.clone();
    view.method = request.method.clone();

    if let Some(body) = &request.body {
        view.body_input = iced::widget::text_editor::Content::with_text(body);
    }

    view.headers_editor.entries = request
        .headers
        .iter()
        .enumerate()
        .map(|(i, (k, v))| KeyValueEntry {
            id: i,
            key: k.clone(),
            value: v.clone(),
        })
        .collect();

    let parsed_url = reqwest::Url::parse(&request.url).ok();
    let query_params: Vec<(String, String)> = parsed_url
        .as_ref()
        .map(|u| {
            u.query_pairs()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect()
        })
        .unwrap_or_default();

    view.params_editor.entries = query_params
        .into_iter()
        .enumerate()
        .map(|(i, (k, v))| KeyValueEntry {
            id: i,
            key: k,
            value: v,
        })
        .collect();

    view.request_config = request.config.clone();

    if !request.multipart_fields.is_empty() {
        view.body_type = BodyType::Multipart;
        view.restore_multipart(&request.multipart_fields);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::http_client::config::RequestConfig;

    fn make_history_entry(method: &str, url: &str) -> RequestHistoryEntry {
        RequestHistoryEntry {
            id: 1,
            method: method.to_string(),
            url: url.to_string(),
            status: Some(200),
            duration_ms: Some(100),
            timestamp: "1234567890".to_string(),
            request_data: None,
            response_data: None,
        }
    }

    fn make_history_entry_with_data(method: &str, url: &str) -> RequestHistoryEntry {
        let request = HttpRequest {
            method: method.to_string(),
            url: url.to_string(),
            headers: vec![("Accept".to_string(), "application/json".to_string())],
            body: Some(r#"{"key":"value"}"#.to_string()),
            config: RequestConfig::default(),
            multipart_fields: vec![],
            auth: None,
        };
        RequestHistoryEntry {
            id: 1,
            method: method.to_string(),
            url: url.to_string(),
            status: Some(201),
            duration_ms: Some(150),
            timestamp: "1234567890".to_string(),
            request_data: serde_json::to_string(&request).ok(),
            response_data: None,
        }
    }

    #[test]
    fn build_view_from_history_without_data() {
        let entry = make_history_entry("GET", "https://example.com");
        let view = build_view_from_history(&entry).unwrap();
        assert_eq!(view.url_input, "https://example.com");
        assert_eq!(view.method, "GET");
    }

    #[test]
    fn build_view_from_history_with_data() {
        let entry = make_history_entry_with_data("POST", "https://api.example.com");
        let view = build_view_from_history(&entry).unwrap();
        assert_eq!(view.url_input, "https://api.example.com");
        assert_eq!(view.method, "POST");
        assert!(view.body_input.text().contains("key"));
        assert_eq!(view.headers_editor.entries.len(), 1);
    }

    #[test]
    fn build_view_from_collection_request_basic() {
        let req = CollectionRequest {
            id: 1,
            collection_id: 1,
            folder_id: None,
            name: "Get Users".to_string(),
            method: "GET".to_string(),
            url: "https://api.example.com/users".to_string(),
            headers: vec![],
            body: None,
            body_type: "text".to_string(),
            auth_type: "none".to_string(),
            auth_data: None,
            params: vec![],
            config_json: None,
            sort_order: 0,
        };
        let view = build_view_from_collection_request(&req);
        assert_eq!(view.url_input, "https://api.example.com/users");
        assert_eq!(view.method, "GET");
    }

    #[test]
    fn build_view_from_collection_request_with_auth() {
        let req = CollectionRequest {
            id: 1,
            collection_id: 1,
            folder_id: None,
            name: "Protected".to_string(),
            method: "GET".to_string(),
            url: "https://api.example.com/protected".to_string(),
            headers: vec![],
            body: None,
            body_type: "text".to_string(),
            auth_type: "bearer".to_string(),
            auth_data: Some("my-token".to_string()),
            params: vec![],
            config_json: None,
            sort_order: 0,
        };
        let view = build_view_from_collection_request(&req);
        match &view.auth {
            crate::data::auth::Auth::BearerToken(token) => assert_eq!(token, "my-token"),
            _ => panic!("Expected BearerToken auth"),
        }
    }

    #[test]
    fn build_view_from_collection_request_with_basic_auth() {
        let req = CollectionRequest {
            id: 1,
            collection_id: 1,
            folder_id: None,
            name: "Basic Auth".to_string(),
            method: "GET".to_string(),
            url: "https://api.example.com".to_string(),
            headers: vec![],
            body: None,
            body_type: "text".to_string(),
            auth_type: "basic".to_string(),
            auth_data: Some("admin:secret".to_string()),
            params: vec![],
            config_json: None,
            sort_order: 0,
        };
        let view = build_view_from_collection_request(&req);
        match &view.auth {
            crate::data::auth::Auth::Basic { user, pass } => {
                assert_eq!(user, "admin");
                assert_eq!(pass, "secret");
            }
            _ => panic!("Expected Basic auth"),
        }
    }

    #[test]
    fn build_view_from_collection_request_with_api_key_auth() {
        let req = CollectionRequest {
            id: 1,
            collection_id: 1,
            folder_id: None,
            name: "API Key".to_string(),
            method: "GET".to_string(),
            url: "https://api.example.com".to_string(),
            headers: vec![],
            body: None,
            body_type: "text".to_string(),
            auth_type: "api_key".to_string(),
            auth_data: Some("X-API-Key:abc123".to_string()),
            params: vec![],
            config_json: None,
            sort_order: 0,
        };
        let view = build_view_from_collection_request(&req);
        match &view.auth {
            crate::data::auth::Auth::ApiKey { key, value, .. } => {
                assert_eq!(key, "X-API-Key");
                assert_eq!(value, "abc123");
            }
            _ => panic!("Expected API Key auth"),
        }
    }

    #[test]
    fn build_view_from_collection_request_with_digest_auth() {
        let req = CollectionRequest {
            id: 1,
            collection_id: 1,
            folder_id: None,
            name: "Digest".to_string(),
            method: "GET".to_string(),
            url: "https://api.example.com".to_string(),
            headers: vec![],
            body: None,
            body_type: "text".to_string(),
            auth_type: "digest".to_string(),
            auth_data: Some("admin:secret".to_string()),
            params: vec![],
            config_json: None,
            sort_order: 0,
        };
        let view = build_view_from_collection_request(&req);
        match &view.auth {
            crate::data::auth::Auth::Digest { user, pass } => {
                assert_eq!(user, "admin");
                assert_eq!(pass, "secret");
            }
            _ => panic!("Expected Digest auth"),
        }
    }

    #[test]
    fn build_view_from_collection_request_with_oauth2_json() {
        let config = crate::data::auth::OAuth2Config {
            grant_type: crate::data::auth::OAuth2GrantType::DeviceCode,
            auth_url: "https://auth.example.com".to_string(),
            token_url: "https://token.example.com".to_string(),
            client_id: "my-client".to_string(),
            scopes: "read write".to_string(),
            access_token: "my-access-token".to_string(),
            refresh_token: "my-refresh-token".to_string(),
            ..Default::default()
        };
        let auth = crate::data::auth::Auth::OAuth2(Box::new(config));
        let auth_json = serde_json::to_string(&auth).unwrap();

        let req = CollectionRequest {
            id: 1,
            collection_id: 1,
            folder_id: None,
            name: "OAuth2".to_string(),
            method: "GET".to_string(),
            url: "https://api.example.com".to_string(),
            headers: vec![],
            body: None,
            body_type: "text".to_string(),
            auth_type: "oauth2".to_string(),
            auth_data: Some(auth_json),
            params: vec![],
            config_json: None,
            sort_order: 0,
        };
        let view = build_view_from_collection_request(&req);
        match &view.auth {
            crate::data::auth::Auth::OAuth2(config) => {
                assert_eq!(config.access_token, "my-access-token");
                assert_eq!(config.refresh_token, "my-refresh-token");
                assert_eq!(config.client_id, "my-client");
                assert_eq!(config.scopes, "read write");
                assert!(matches!(
                    config.grant_type,
                    crate::data::auth::OAuth2GrantType::DeviceCode
                ));
            }
            _ => panic!("Expected OAuth2 auth"),
        }
    }

    #[test]
    fn build_view_from_collection_request_with_oauth2_legacy_string() {
        let req = CollectionRequest {
            id: 1,
            collection_id: 1,
            folder_id: None,
            name: "OAuth2".to_string(),
            method: "GET".to_string(),
            url: "https://api.example.com".to_string(),
            headers: vec![],
            body: None,
            body_type: "text".to_string(),
            auth_type: "oauth2".to_string(),
            auth_data: Some("my-access-token".to_string()),
            params: vec![],
            config_json: None,
            sort_order: 0,
        };
        let view = build_view_from_collection_request(&req);
        match &view.auth {
            crate::data::auth::Auth::OAuth2(config) => {
                assert_eq!(config.access_token, "my-access-token");
                assert_eq!(config.refresh_token, "");
            }
            _ => panic!("Expected OAuth2 auth"),
        }
    }

    #[test]
    fn build_view_from_collection_request_restores_config() {
        use crate::http_client::config::RequestConfig;

        let req = CollectionRequest {
            id: 1,
            collection_id: 1,
            folder_id: None,
            name: "With Config".to_string(),
            method: "GET".to_string(),
            url: "https://api.example.com".to_string(),
            headers: vec![],
            body: None,
            body_type: "text".to_string(),
            auth_type: "none".to_string(),
            auth_data: None,
            params: vec![],
            config_json: None,
            sort_order: 0,
        };
        let view = build_view_from_collection_request(&req);
        assert_eq!(view.request_config, RequestConfig::default());
    }

    #[test]
    fn build_view_from_collection_request_with_multipart() {
        let req = CollectionRequest {
            id: 1,
            collection_id: 1,
            folder_id: None,
            name: "Upload".to_string(),
            method: "POST".to_string(),
            url: "https://api.example.com/upload".to_string(),
            headers: vec![],
            body: None,
            body_type: "multipart".to_string(),
            auth_type: "none".to_string(),
            auth_data: None,
            params: vec![],
            config_json: None,
            sort_order: 0,
        };
        let view = build_view_from_collection_request(&req);
        assert_eq!(view.body_type, BodyType::Multipart);
    }

    #[test]
    fn build_view_from_collection_request_with_headers_and_params() {
        let req = CollectionRequest {
            id: 1,
            collection_id: 1,
            folder_id: None,
            name: "Full".to_string(),
            method: "GET".to_string(),
            url: "https://api.example.com".to_string(),
            headers: vec![("X-Custom".to_string(), "value".to_string())],
            body: None,
            body_type: "text".to_string(),
            auth_type: "none".to_string(),
            auth_data: None,
            params: vec![("key".to_string(), "val".to_string())],
            config_json: None,
            sort_order: 0,
        };
        let view = build_view_from_collection_request(&req);
        assert_eq!(view.headers_editor.entries.len(), 1);
        assert_eq!(view.params_editor.entries.len(), 1);
    }

    #[test]
    fn oauth2_json_round_trip_preserves_all_fields() {
        let original_auth =
            crate::data::auth::Auth::OAuth2(Box::new(crate::data::auth::OAuth2Config {
                grant_type: crate::data::auth::OAuth2GrantType::DeviceCode,
                auth_url: "https://auth.example.com/authorize".to_string(),
                token_url: "https://token.example.com/token".to_string(),
                device_auth_url: "https://device.example.com".to_string(),
                client_id: "my-client-id".to_string(),
                client_secret: "my-secret".to_string(),
                scopes: "read write admin".to_string(),
                redirect_uri: "http://localhost:8080/callback".to_string(),
                pkce_enabled: true,
                pkce_verifier: None,
                access_token: "eyJhbGciOiJSUzI1NiIs...".to_string(),
                refresh_token: "dGhpcyBpcyBhIHJlZnJlc2g...".to_string(),
                token_expiry: Some("2025-12-31T23:59:59Z".to_string()),
                device_code: "ABC-123-DEF".to_string(),
                user_code: "ABCD-1234".to_string(),
                verification_uri: "https://device.example.com/verify".to_string(),
                device_code_expires_in: Some(300),
                device_code_interval: Some(5),
                status: crate::data::auth::OAuth2Status::default(),
            }));

        let json = serde_json::to_string(&original_auth).unwrap();
        let req = CollectionRequest {
            id: 1,
            collection_id: 1,
            folder_id: None,
            name: "Full OAuth2".to_string(),
            method: "POST".to_string(),
            url: "https://api.example.com/resource".to_string(),
            headers: vec![],
            body: None,
            body_type: "text".to_string(),
            auth_type: "oauth2".to_string(),
            auth_data: Some(json),
            params: vec![],
            config_json: None,
            sort_order: 0,
        };
        let view = build_view_from_collection_request(&req);
        match &view.auth {
            crate::data::auth::Auth::OAuth2(config) => {
                assert_eq!(config.auth_url, "https://auth.example.com/authorize");
                assert_eq!(config.token_url, "https://token.example.com/token");
                assert_eq!(config.device_auth_url, "https://device.example.com");
                assert_eq!(config.client_id, "my-client-id");
                assert_eq!(config.client_secret, "my-secret");
                assert_eq!(config.scopes, "read write admin");
                assert_eq!(config.redirect_uri, "http://localhost:8080/callback");
                assert!(config.pkce_enabled);
                assert!(config.access_token.starts_with("eyJ"));
                assert!(config.refresh_token.starts_with("dGhpc"));
                assert_eq!(config.token_expiry.as_deref(), Some("2025-12-31T23:59:59Z"));
                assert_eq!(config.device_code, "ABC-123-DEF");
                assert_eq!(config.user_code, "ABCD-1234");
                assert_eq!(config.verification_uri, "https://device.example.com/verify");
                assert_eq!(config.device_code_expires_in, Some(300));
                assert_eq!(config.device_code_interval, Some(5));
                assert!(matches!(
                    config.grant_type,
                    crate::data::auth::OAuth2GrantType::DeviceCode
                ));
            }
            _ => panic!("Expected OAuth2 auth, got {:?}", view.auth),
        }
    }

    #[test]
    fn api_key_json_round_trip_preserves_location() {
        let original_auth = crate::data::auth::Auth::ApiKey {
            key: "X-API-Key".to_string(),
            value: "secret-123".to_string(),
            location: crate::data::auth::ApiKeyLocation::Query,
        };
        let json = serde_json::to_string(&original_auth).unwrap();

        let req = CollectionRequest {
            id: 1,
            collection_id: 1,
            folder_id: None,
            name: "API Key".to_string(),
            method: "GET".to_string(),
            url: "https://api.example.com".to_string(),
            headers: vec![],
            body: None,
            body_type: "text".to_string(),
            auth_type: "api_key".to_string(),
            auth_data: Some(json),
            params: vec![],
            config_json: None,
            sort_order: 0,
        };
        let view = build_view_from_collection_request(&req);
        match &view.auth {
            crate::data::auth::Auth::ApiKey {
                key,
                value,
                location,
            } => {
                assert_eq!(key, "X-API-Key");
                assert_eq!(value, "secret-123");
                assert_eq!(*location, crate::data::auth::ApiKeyLocation::Query);
            }
            _ => panic!("Expected ApiKey auth"),
        }
    }
}
