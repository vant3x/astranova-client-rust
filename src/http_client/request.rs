#[derive(Debug, Clone)]
pub struct HttpRequest {
    pub method: String,
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: Option<String>,
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
        };
        assert_eq!(req.method, "GET");
        assert_eq!(req.url, "https://example.com");
        assert!(req.headers.is_empty());
        assert!(req.body.is_none());
    }

    #[test]
    fn create_post_request_with_body() {
        let req = HttpRequest {
            method: "POST".to_string(),
            url: "https://example.com/api".to_string(),
            headers: vec![("Content-Type".to_string(), "application/json".to_string())],
            body: Some(r#"{"key": "value"}"#.to_string()),
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
        };
        assert_eq!(req.headers.len(), 3);
    }
}
