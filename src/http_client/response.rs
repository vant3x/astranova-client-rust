use crate::http_client::request::HttpMethod;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum BodyEncoding {
    #[default]
    Text,
    Base64,
}

impl std::fmt::Display for BodyEncoding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Text => write!(f, "text"),
            Self::Base64 => write!(f, "base64"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpResponse {
    pub url: String,
    pub method: HttpMethod,
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: String,
    #[serde(default)]
    pub body_encoding: BodyEncoding,
    #[serde(with = "duration_millis")]
    pub duration: Duration,
    pub size: u64,
    pub redirect_chain: Vec<String>,
}

mod duration_millis {
    use serde::{self, Deserialize, Deserializer, Serializer};
    use std::time::Duration;

    pub fn serialize<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u64(duration.as_millis() as u64)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        let millis = u64::deserialize(deserializer)?;
        Ok(Duration::from_millis(millis))
    }
}

/// Events emitted during a streaming HTTP response.
#[derive(Debug, Clone)]
pub enum HttpStreamEvent {
    /// Response headers received (status + headers).
    HeadersReceived {
        status: u16,
        headers: Vec<(String, String)>,
        url: String,
        #[allow(dead_code)]
        method: String,
    },
    /// A chunk of the response body arrived.
    BodyChunk(Vec<u8>),
    /// The response body is binary (base64-encoded in the final response).
    BodyChunkBinary(Vec<u8>),
    /// The stream finished successfully. Contains the total size.
    StreamComplete { total_size: u64 },
    /// The stream failed.
    StreamError(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::http_client::request::HttpMethod;

    #[test]
    fn create_response() {
        let resp = HttpResponse {
            url: "https://example.com".to_string(),
            method: HttpMethod::Get,
            status: 200,
            headers: vec![],
            body: "OK".to_string(),
            body_encoding: BodyEncoding::Text,
            duration: Duration::from_millis(150),
            size: 2,
            redirect_chain: vec![],
        };
        assert_eq!(resp.status, 200);
        assert_eq!(resp.size, 2);
        assert_eq!(resp.duration, Duration::from_millis(150));
        assert_eq!(resp.body_encoding, BodyEncoding::Text);
    }

    #[test]
    fn response_clone() {
        let resp = HttpResponse {
            url: "https://example.com".to_string(),
            method: HttpMethod::Post,
            status: 201,
            headers: vec![("Location".to_string(), "/resource/1".to_string())],
            body: r#"{"id": 1}"#.to_string(),
            body_encoding: BodyEncoding::Text,
            duration: Duration::from_millis(200),
            size: 13,
            redirect_chain: vec!["https://old.example.com".to_string()],
        };
        let cloned = resp.clone();
        assert_eq!(resp.status, cloned.status);
        assert_eq!(resp.body, cloned.body);
        assert_eq!(resp.headers, cloned.headers);
        assert_eq!(resp.redirect_chain, cloned.redirect_chain);
    }

    #[test]
    fn response_status_codes() {
        let statuses = [200, 201, 301, 400, 404, 500, 503];
        for status in statuses {
            let resp = HttpResponse {
                url: String::new(),
                method: HttpMethod::Get,
                status,
                headers: vec![],
                body: String::new(),
                body_encoding: BodyEncoding::default(),
                duration: Duration::ZERO,
                size: 0,
                redirect_chain: vec![],
            };
            assert_eq!(resp.status, status);
        }
    }

    #[test]
    fn response_with_redirect_chain() {
        let resp = HttpResponse {
            url: "https://final.example.com".to_string(),
            method: HttpMethod::Get,
            status: 200,
            headers: vec![],
            body: String::new(),
            body_encoding: BodyEncoding::default(),
            duration: Duration::ZERO,
            size: 0,
            redirect_chain: vec![
                "https://old.example.com".to_string(),
                "https://intermediate.example.com".to_string(),
            ],
        };
        assert_eq!(resp.redirect_chain.len(), 2);
    }

    #[test]
    fn serialize_response_to_json() {
        let resp = HttpResponse {
            url: "https://api.example.com".to_string(),
            method: HttpMethod::Get,
            status: 200,
            headers: vec![("Content-Type".to_string(), "application/json".to_string())],
            body: r#"{"ok": true}"#.to_string(),
            body_encoding: BodyEncoding::Text,
            duration: Duration::from_millis(150),
            size: 14,
            redirect_chain: vec![],
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"status\":200"));
        assert!(json.contains("\"duration\":150"));
        assert!(json.contains("Content-Type"));
    }

    #[test]
    fn deserialize_response_from_json() {
        let json = r#"{
            "url": "https://api.example.com/data",
            "method": "POST",
            "status": 201,
            "headers": [["Location", "/resource/1"]],
            "body": "{\"id\": 1}",
            "duration": 200,
            "size": 13,
            "redirect_chain": ["https://old.example.com"]
        }"#;
        let resp: HttpResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.status, 201);
        assert_eq!(resp.duration, Duration::from_millis(200));
        assert_eq!(resp.redirect_chain.len(), 1);
        assert_eq!(resp.body_encoding, BodyEncoding::Text);
    }

    #[test]
    fn roundtrip_response_serialization() {
        let resp = HttpResponse {
            url: "https://api.example.com".to_string(),
            method: HttpMethod::Delete,
            status: 204,
            headers: vec![],
            body: String::new(),
            body_encoding: BodyEncoding::default(),
            duration: Duration::from_millis(50),
            size: 0,
            redirect_chain: vec!["https://old.example.com".to_string()],
        };
        let json = serde_json::to_string(&resp).unwrap();
        let deserialized: HttpResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(resp.status, deserialized.status);
        assert_eq!(resp.duration, deserialized.duration);
        assert_eq!(resp.redirect_chain, deserialized.redirect_chain);
    }

    #[test]
    fn body_encoding_display() {
        assert_eq!(BodyEncoding::Text.to_string(), "text");
        assert_eq!(BodyEncoding::Base64.to_string(), "base64");
    }

    #[test]
    fn body_encoding_default_is_text() {
        assert_eq!(BodyEncoding::default(), BodyEncoding::Text);
    }
}
