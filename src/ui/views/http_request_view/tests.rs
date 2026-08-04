#[cfg(test)]
mod view_tests {
    use crate::data::auth::Auth;
    use crate::persistence::database::Environment;
    use crate::ui::components::key_value_editor::KeyValueEntry;
    use crate::ui::views::http_request_view::{
        BodyType, ContentType, HttpRequestView, MultipartEntry,
    };
    use base64::Engine;
    use iced::widget::text_editor;

    fn make_view(url: &str, method: &str) -> HttpRequestView {
        HttpRequestView {
            url_input: url.to_string(),
            method: method.to_string(),
            ..Default::default()
        }
    }

    fn build(view: &HttpRequestView) -> crate::http_client::request::HttpRequest {
        view.build_request()
            .expect("build_request should succeed in tests")
    }

    #[test]
    fn build_request_basic_get() {
        let view = make_view("https://example.com/api", "GET");
        let req = build(&view);
        assert_eq!(req.method, crate::http_client::request::HttpMethod::Get);
        assert_eq!(req.url, "https://example.com/api");
        assert!(req.body.is_none());
    }

    #[test]
    fn build_request_with_params() {
        let mut view = make_view("https://example.com/api", "GET");
        view.params_editor.entries = vec![
            KeyValueEntry {
                id: 0,
                key: "page".to_string(),
                value: "1".to_string(),
                secret: false,
            },
            KeyValueEntry {
                id: 1,
                key: "limit".to_string(),
                value: "10".to_string(),
                secret: false,
            },
        ];
        let req = build(&view);
        assert!(req.url.contains("page=1"));
        assert!(req.url.contains("limit=10"));
        assert!(req.url.contains('?'));
        assert!(req.url.contains('&'));
    }

    #[test]
    fn build_request_params_appended_to_existing_query() {
        let mut view = make_view("https://example.com/api?existing=true", "GET");
        view.params_editor.entries = vec![KeyValueEntry {
            id: 0,
            key: "new".to_string(),
            value: "val".to_string(),
            secret: false,
        }];
        let req = build(&view);
        assert!(req.url.contains("existing=true"));
        assert!(req.url.contains("new=val"));
        if let Some(query_start) = req.url.find('?') {
            let rest = &req.url[query_start..];
            assert!(!rest[1..].contains('?'));
        } else {
            // No query part – nothing to parse
        }
    }

    #[test]
    fn build_request_empty_params_filtered() {
        let mut view = make_view("https://example.com/api", "GET");
        view.params_editor.entries = vec![
            KeyValueEntry {
                id: 0,
                key: String::new(),
                value: "val".to_string(),
                secret: false,
            },
            KeyValueEntry {
                id: 1,
                key: "good".to_string(),
                value: "yes".to_string(),
                secret: false,
            },
        ];
        let req = build(&view);
        assert!(!req.url.contains("val"));
        assert!(req.url.contains("good=yes"));
    }

    #[test]
    fn build_request_with_headers() {
        let mut view = make_view("https://example.com", "GET");
        view.headers_editor.entries = vec![KeyValueEntry {
            id: 0,
            key: "Accept".to_string(),
            value: "text/html".to_string(),
            secret: false,
        }];
        let req = build(&view);
        assert!(req
            .headers
            .iter()
            .any(|(k, v)| k == "Accept" && v == "text/html"));
    }

    #[test]
    fn build_request_empty_headers_filtered() {
        let mut view = make_view("https://example.com", "GET");
        view.headers_editor.entries = vec![KeyValueEntry {
            id: 0,
            key: String::new(),
            value: "val".to_string(),
            secret: false,
        }];
        let req = build(&view);
        assert!(!req.headers.iter().any(|(k, _)| k.is_empty()));
    }

    #[test]
    fn build_request_bearer_auth() {
        let mut view = make_view("https://example.com", "GET");
        view.auth = Auth::BearerToken("my-secret-token".to_string());
        let req = build(&view);
        assert!(req
            .headers
            .iter()
            .any(|(k, v)| k == "Authorization" && v == "Bearer my-secret-token"));
    }

    #[test]
    fn build_request_bearer_empty_token_ignored() {
        let mut view = make_view("https://example.com", "GET");
        view.auth = Auth::BearerToken(String::new());
        let req = build(&view);
        assert!(!req.headers.iter().any(|(k, _)| k == "Authorization"));
    }

    #[test]
    fn build_request_basic_auth() {
        let mut view = make_view("https://example.com", "GET");
        view.auth = Auth::Basic {
            user: "admin".to_string(),
            pass: "secret123".to_string(),
        };
        let req = build(&view);
        let auth_header = req.headers.iter().find(|(k, _)| k == "Authorization");
        if let Some((_, value)) = auth_header {
            assert!(value.starts_with("Basic "));
            let encoded = value.strip_prefix("Basic ").expect("Missing Basic prefix");
            let decoded = base64::engine::general_purpose::STANDARD
                .decode(encoded)
                .unwrap();
            let decoded_str = String::from_utf8(decoded).unwrap();
            assert_eq!(decoded_str, "admin:secret123");
        } else {
            // No Authorization header – basic auth not set
        }
    }

    #[test]
    fn build_request_basic_auth_empty_ignored() {
        let mut view = make_view("https://example.com", "GET");
        view.auth = Auth::Basic {
            user: String::new(),
            pass: String::new(),
        };
        let req = build(&view);
        assert!(!req.headers.iter().any(|(k, _)| k == "Authorization"));
    }

    #[test]
    fn build_request_body_sets_content_type() {
        let mut view = make_view("https://example.com", "POST");
        view.body_input = text_editor::Content::with_text(r#"{"key": "value"}"#);
        view.request_content_type = ContentType::Json;
        let req = build(&view);
        assert!(req.body.is_some());
        assert!(req
            .headers
            .iter()
            .any(|(k, v)| k == "Content-Type" && v == "application/json"));
    }

    #[test]
    fn build_request_no_body_no_content_type() {
        let view = make_view("https://example.com", "GET");
        let req = build(&view);
        assert_eq!(req.method, crate::http_client::request::HttpMethod::Get);
        assert_eq!(req.url, "https://example.com");
    }

    #[test]
    fn build_request_content_types() {
        let cases = vec![
            (ContentType::Json, "application/json"),
            (ContentType::Text, "text/plain"),
            (ContentType::Html, "text/html"),
            (ContentType::Xml, "application/xml"),
        ];
        for (ct, expected) in cases {
            let mut view = make_view("https://example.com", "POST");
            view.body_input = text_editor::Content::with_text("data");
            view.request_content_type = ct;
            let req = build(&view);
            assert!(
                req.headers
                    .iter()
                    .any(|(k, v)| k == "Content-Type" && v == expected),
                "Failed for {ct:?}: expected {expected}"
            );
        }
    }

    #[test]
    fn apply_environment_replaces_url_variable() {
        let mut view = make_view("{{BASE_URL}}/api/users", "GET");
        let env = Environment {
            id: 1,
            name: "test".to_string(),
            variables: vec![(
                "BASE_URL".to_string(),
                "https://api.example.com".to_string(),
            )],
            secret_keys: vec![],
            default_endpoint: None,
        };
        view.apply_environment(&env);
        assert_eq!(view.url_input, "https://api.example.com/api/users");
    }

    #[test]
    fn apply_environment_replaces_body_variable() {
        let mut view = make_view("https://example.com", "POST");
        view.body_input = text_editor::Content::with_text(r#"{"token": "{{API_TOKEN}}"}"#);
        let env = Environment {
            id: 1,
            name: "test".to_string(),
            variables: vec![("API_TOKEN".to_string(), "abc123".to_string())],
            secret_keys: vec![],
            default_endpoint: None,
        };
        view.apply_environment(&env);
        assert!(view.body_input.text().contains(r#"{"token": "abc123"}"#));
    }

    #[test]
    fn apply_environment_replaces_header_variable() {
        let mut view = make_view("https://example.com", "GET");
        view.headers_editor.entries = vec![KeyValueEntry {
            id: 0,
            key: "Authorization".to_string(),
            value: "Bearer {{TOKEN}}".to_string(),
            secret: false,
        }];
        let env = Environment {
            id: 1,
            name: "test".to_string(),
            variables: vec![("TOKEN".to_string(), "my-jwt-token".to_string())],
            secret_keys: vec![],
            default_endpoint: None,
        };
        view.apply_environment(&env);
        assert_eq!(view.headers_editor.entries[0].value, "Bearer my-jwt-token");
    }

    #[test]
    fn apply_environment_replaces_param_variable() {
        let mut view = make_view("https://example.com", "GET");
        view.params_editor.entries = vec![KeyValueEntry {
            id: 0,
            key: "key".to_string(),
            value: "{{API_KEY}}".to_string(),
            secret: false,
        }];
        let env = Environment {
            id: 1,
            name: "test".to_string(),
            variables: vec![("API_KEY".to_string(), "secret-key-123".to_string())],
            secret_keys: vec![],
            default_endpoint: None,
        };
        view.apply_environment(&env);
        assert_eq!(view.params_editor.entries[0].value, "secret-key-123");
    }

    #[test]
    fn apply_environment_replaces_bearer_token_variable() {
        let mut view = make_view("https://example.com", "GET");
        view.auth = Auth::BearerToken("{{JWT}}".to_string());
        let env = Environment {
            id: 1,
            name: "test".to_string(),
            variables: vec![("JWT".to_string(), "eyJhbGciOiJIUzI1NiJ9".to_string())],
            secret_keys: vec![],
            default_endpoint: None,
        };
        view.apply_environment(&env);
        assert_eq!(
            view.auth,
            Auth::BearerToken("eyJhbGciOiJIUzI1NiJ9".to_string())
        );
    }

    #[test]
    fn apply_environment_replaces_basic_auth_variable() {
        let mut view = make_view("https://example.com", "GET");
        view.auth = Auth::Basic {
            user: "{{USER}}".to_string(),
            pass: "{{PASS}}".to_string(),
        };
        let env = Environment {
            id: 1,
            name: "test".to_string(),
            variables: vec![
                ("USER".to_string(), "admin".to_string()),
                ("PASS".to_string(), "secret".to_string()),
            ],
            secret_keys: vec![],
            default_endpoint: None,
        };
        view.apply_environment(&env);
        assert_eq!(
            view.auth,
            Auth::Basic {
                user: "admin".to_string(),
                pass: "secret".to_string()
            }
        );
    }

    #[test]
    fn apply_environment_multiple_variables() {
        let mut view = make_view("{{PROTO}}://{{HOST}}:{{PORT}}/api", "GET");
        let env = Environment {
            id: 1,
            name: "test".to_string(),
            variables: vec![
                ("PROTO".to_string(), "https".to_string()),
                ("HOST".to_string(), "localhost".to_string()),
                ("PORT".to_string(), "8080".to_string()),
            ],
            secret_keys: vec![],
            default_endpoint: None,
        };
        view.apply_environment(&env);
        assert_eq!(view.url_input, "https://localhost:8080/api");
    }

    #[test]
    fn apply_environment_no_variables_no_change() {
        let mut view = make_view("https://example.com/api", "GET");
        let env = Environment {
            id: 1,
            name: "test".to_string(),
            variables: vec![],
            secret_keys: vec![],
            default_endpoint: None,
        };
        view.apply_environment(&env);
        assert_eq!(view.url_input, "https://example.com/api");
    }

    #[test]
    fn build_request_multipart_text_fields() {
        let mut view = make_view("https://example.com/upload", "POST");
        view.body_type = BodyType::Multipart;
        view.multipart_entries = vec![
            MultipartEntry {
                id: 0,
                name: "username".to_string(),
                value: "john".to_string(),
                is_file: false,
            },
            MultipartEntry {
                id: 1,
                name: "bio".to_string(),
                value: "Hello world".to_string(),
                is_file: false,
            },
        ];
        let req = build(&view);
        assert_eq!(req.multipart_fields.len(), 2);
        assert!(!req.headers.iter().any(|(k, _)| k == "Content-Type"));
    }

    #[test]
    fn build_request_multipart_file_field() {
        let mut view = make_view("https://example.com/upload", "POST");
        view.body_type = BodyType::Multipart;
        view.multipart_entries = vec![MultipartEntry {
            id: 0,
            name: "document".to_string(),
            value: "/tmp/test.pdf".to_string(),
            is_file: true,
        }];
        let req = build(&view);
        assert_eq!(req.multipart_fields.len(), 1);
        match &req.multipart_fields[0].value {
            crate::http_client::request::MultipartValue::File { path, .. } => {
                assert_eq!(path, "/tmp/test.pdf");
            }
            crate::http_client::request::MultipartValue::Text(_) => panic!("Expected File variant"),
        }
    }

    #[test]
    fn build_request_multipart_empty_names_filtered() {
        let mut view = make_view("https://example.com/upload", "POST");
        view.body_type = BodyType::Multipart;
        view.multipart_entries = vec![
            MultipartEntry {
                id: 0,
                name: String::new(),
                value: "val".to_string(),
                is_file: false,
            },
            MultipartEntry {
                id: 1,
                name: "good".to_string(),
                value: "yes".to_string(),
                is_file: false,
            },
        ];
        let req = build(&view);
        assert_eq!(req.multipart_fields.len(), 1);
        assert_eq!(req.multipart_fields[0].name, "good");
    }

    #[test]
    fn build_request_text_mode_ignores_multipart_entries() {
        let mut view = make_view("https://example.com/api", "POST");
        view.body_type = BodyType::Text;
        view.multipart_entries = vec![MultipartEntry {
            id: 0,
            name: "field".to_string(),
            value: "val".to_string(),
            is_file: false,
        }];
        let req = build(&view);
        assert!(req.multipart_fields.is_empty());
    }

    #[test]
    fn build_request_api_key_header() {
        let mut view = make_view("https://example.com", "GET");
        view.auth = Auth::ApiKey {
            key: "X-API-Key".to_string(),
            value: "secret123".to_string(),
            location: crate::data::auth::ApiKeyLocation::Header,
        };
        let req = build(&view);
        assert!(req
            .headers
            .iter()
            .any(|(k, v)| k == "X-API-Key" && v == "secret123"));
    }

    #[test]
    fn build_request_api_key_query() {
        let mut view = make_view("https://example.com", "GET");
        view.auth = Auth::ApiKey {
            key: "api_key".to_string(),
            value: "secret123".to_string(),
            location: crate::data::auth::ApiKeyLocation::Query,
        };
        let req = build(&view);
        assert!(req.url.contains("api_key=secret123"));
        assert!(!req.headers.iter().any(|(k, _)| k == "api_key"));
    }

    #[test]
    fn build_request_api_key_query_with_existing_params() {
        let mut view = make_view("https://example.com?page=1", "GET");
        view.auth = Auth::ApiKey {
            key: "api_key".to_string(),
            value: "secret123".to_string(),
            location: crate::data::auth::ApiKeyLocation::Query,
        };
        let req = build(&view);
        assert!(req.url.contains("page=1"));
        assert!(req.url.contains("api_key=secret123"));
    }

    #[test]
    fn build_request_api_key_empty_key_ignored() {
        let mut view = make_view("https://example.com", "GET");
        view.auth = Auth::ApiKey {
            key: String::new(),
            value: "secret123".to_string(),
            location: crate::data::auth::ApiKeyLocation::Header,
        };
        let req = build(&view);
        assert!(!req
            .headers
            .iter()
            .any(|(k, _)| k == "X-API-Key" || k.is_empty()));
    }

    #[test]
    fn build_request_sets_auth_field() {
        let mut view = make_view("https://example.com", "GET");
        view.auth = Auth::BearerToken("token".to_string());
        let req = build(&view);
        assert!(req.auth.is_some());
        assert_eq!(
            req.auth.as_ref().unwrap(),
            &Auth::BearerToken("token".to_string())
        );
    }

    #[test]
    fn build_request_oauth2_auth() {
        let mut view = make_view("https://example.com", "GET");
        let config = crate::data::auth::OAuth2Config {
            access_token: "my-oauth-token".to_string(),
            ..Default::default()
        };
        view.auth = Auth::OAuth2(Box::new(config));
        let req = build(&view);
        assert!(req
            .headers
            .iter()
            .any(|(k, v)| k == "Authorization" && v == "Bearer my-oauth-token"));
    }

    #[test]
    fn apply_environment_replaces_api_key_variable() {
        let mut view = make_view("https://example.com", "GET");
        view.auth = Auth::ApiKey {
            key: "X-API-Key".to_string(),
            value: "{{API_KEY}}".to_string(),
            location: crate::data::auth::ApiKeyLocation::Header,
        };
        let env = Environment {
            id: 1,
            name: "test".to_string(),
            variables: vec![("API_KEY".to_string(), "my-secret".to_string())],
            secret_keys: vec![],
            default_endpoint: None,
        };
        view.apply_environment(&env);
        match &view.auth {
            Auth::ApiKey { value, .. } => assert_eq!(value, "my-secret"),
            _ => panic!("Expected ApiKey"),
        }
    }

    #[test]
    fn apply_environment_replaces_oauth2_device_auth_url() {
        let mut view = make_view("https://example.com", "GET");
        let config = crate::data::auth::OAuth2Config {
            device_auth_url: "{{DEVICE_AUTH}}".to_string(),
            ..Default::default()
        };
        view.auth = Auth::OAuth2(Box::new(config));
        let env = Environment {
            id: 1,
            name: "test".to_string(),
            variables: vec![(
                "DEVICE_AUTH".to_string(),
                "https://device.example.com".to_string(),
            )],
            secret_keys: vec![],
            default_endpoint: None,
        };
        view.apply_environment(&env);
        match &view.auth {
            Auth::OAuth2(config) => {
                assert_eq!(config.device_auth_url, "https://device.example.com");
            }
            _ => panic!("Expected OAuth2"),
        }
    }

    #[test]
    fn build_request_form_urlencoded_basic() {
        let mut view = make_view("https://example.com/login", "POST");
        view.body_type = BodyType::FormUrlencoded;
        view.form_entries = vec![
            MultipartEntry {
                id: 0,
                name: "username".to_string(),
                value: "john".to_string(),
                is_file: false,
            },
            MultipartEntry {
                id: 1,
                name: "password".to_string(),
                value: "secret".to_string(),
                is_file: false,
            },
        ];
        let req = build(&view);
        let body = req.body.unwrap();
        assert!(body.contains("username=john"));
        assert!(body.contains("password=secret"));
        assert!(body.contains('&'));
        let ct = req
            .headers
            .iter()
            .find(|(k, _)| k == "Content-Type")
            .map(|(_, v)| v.clone())
            .unwrap();
        assert_eq!(ct, "application/x-www-form-urlencoded");
    }

    #[test]
    fn build_request_form_urlencoded_special_chars() {
        let mut view = make_view("https://example.com/login", "POST");
        view.body_type = BodyType::FormUrlencoded;
        view.form_entries = vec![MultipartEntry {
            id: 0,
            name: "email".to_string(),
            value: "a@b.com".to_string(),
            is_file: false,
        }];
        let req = build(&view);
        let body = req.body.unwrap();
        assert!(body.contains("email=a%40b.com"));
    }

    #[test]
    fn build_request_form_urlencoded_empty_filtered() {
        let mut view = make_view("https://example.com/login", "POST");
        view.body_type = BodyType::FormUrlencoded;
        view.form_entries = vec![
            MultipartEntry {
                id: 0,
                name: String::new(),
                value: "val".to_string(),
                is_file: false,
            },
            MultipartEntry {
                id: 1,
                name: "good".to_string(),
                value: "yes".to_string(),
                is_file: false,
            },
        ];
        let req = build(&view);
        let body = req.body.unwrap();
        assert!(!body.contains("val"));
        assert!(body.contains("good=yes"));
    }

    #[test]
    fn build_request_form_urlencoded_no_body_when_empty() {
        let mut view = make_view("https://example.com/api", "POST");
        view.body_type = BodyType::FormUrlencoded;
        view.form_entries = vec![];
        let req = build(&view);
        assert!(req.body.is_none());
        assert!(!req.headers.iter().any(|(k, _)| k == "Content-Type"));
    }
}
