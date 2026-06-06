use super::config::RequestConfig;

#[derive(Debug, Clone)]
pub struct HttpRequest {
    pub method: String,
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: Option<String>,
    pub config: RequestConfig,
    pub multipart_fields: Vec<MultipartField>,
}

#[derive(Debug, Clone)]
pub struct MultipartField {
    pub name: String,
    pub value: MultipartValue,
}

#[derive(Debug, Clone)]
pub enum MultipartValue {
    Text(String),
    File {
        path: String,
        filename: Option<String>,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_get_request() {
        let req = HttpRequest {
            method: "GET".to_string(),
            url: "https://example.com".to_string(),
            headers: vec![],
            body: None,
            config: RequestConfig::default(),
            multipart_fields: vec![],
        };
        assert_eq!(req.method, "GET");
        assert_eq!(req.url, "https://example.com");
        assert!(req.headers.is_empty());
        assert!(req.body.is_none());
        assert!(req.multipart_fields.is_empty());
    }

    #[test]
    fn create_post_request_with_body() {
        let req = HttpRequest {
            method: "POST".to_string(),
            url: "https://example.com/api".to_string(),
            headers: vec![("Content-Type".to_string(), "application/json".to_string())],
            body: Some(r#"{"key": "value"}"#.to_string()),
            config: RequestConfig::default(),
            multipart_fields: vec![],
        };
        assert_eq!(req.method, "POST");
        assert!(req.body.is_some());
        assert_eq!(req.headers.len(), 1);
    }

    #[test]
    fn request_clone() {
        let req = HttpRequest {
            method: "PUT".to_string(),
            url: "https://example.com/1".to_string(),
            headers: vec![("X-Custom".to_string(), "test".to_string())],
            body: Some("data".to_string()),
            config: RequestConfig::default(),
            multipart_fields: vec![],
        };
        let cloned = req.clone();
        assert_eq!(req.method, cloned.method);
        assert_eq!(req.url, cloned.url);
        assert_eq!(req.headers, cloned.headers);
        assert_eq!(req.body, cloned.body);
    }

    #[test]
    fn request_with_multiple_headers() {
        let req = HttpRequest {
            method: "GET".to_string(),
            url: "https://example.com".to_string(),
            headers: vec![
                ("Accept".to_string(), "application/json".to_string()),
                ("Authorization".to_string(), "Bearer token".to_string()),
                ("X-Request-Id".to_string(), "12345".to_string()),
            ],
            body: None,
            config: RequestConfig::default(),
            multipart_fields: vec![],
        };
        assert_eq!(req.headers.len(), 3);
    }

    #[test]
    fn request_with_custom_timeout() {
        use std::time::Duration;
        let config = RequestConfig {
            timeout: Duration::from_secs(60),
            ..Default::default()
        };
        let req = HttpRequest {
            method: "GET".to_string(),
            url: "https://example.com".to_string(),
            headers: vec![],
            body: None,
            config,
            multipart_fields: vec![],
        };
        assert_eq!(req.config.timeout, Duration::from_secs(60));
    }

    #[test]
    fn multipart_field_text() {
        let field = MultipartField {
            name: "username".to_string(),
            value: MultipartValue::Text("john".to_string()),
        };
        assert_eq!(field.name, "username");
        match &field.value {
            MultipartValue::Text(t) => assert_eq!(t, "john"),
            _ => panic!("Expected Text"),
        }
    }

    #[test]
    fn multipart_field_file() {
        let field = MultipartField {
            name: "document".to_string(),
            value: MultipartValue::File {
                path: "/tmp/test.pdf".to_string(),
                filename: Some("test.pdf".to_string()),
            },
        };
        match &field.value {
            MultipartValue::File { path, filename } => {
                assert_eq!(path, "/tmp/test.pdf");
                assert_eq!(filename.as_deref(), Some("test.pdf"));
            }
            _ => panic!("Expected File"),
        }
    }

    #[test]
    fn request_with_multipart_fields() {
        let req = HttpRequest {
            method: "POST".to_string(),
            url: "https://example.com/upload".to_string(),
            headers: vec![],
            body: None,
            config: RequestConfig::default(),
            multipart_fields: vec![
                MultipartField {
                    name: "field1".to_string(),
                    value: MultipartValue::Text("value1".to_string()),
                },
                MultipartField {
                    name: "file1".to_string(),
                    value: MultipartValue::File {
                        path: "/tmp/data.json".to_string(),
                        filename: Some("data.json".to_string()),
                    },
                },
            ],
        };
        assert_eq!(req.multipart_fields.len(), 2);
    }
}
