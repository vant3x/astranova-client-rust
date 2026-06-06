use std::time::Duration;

#[derive(Debug, Clone)]
pub struct HttpResponse {
    pub url: String,
    pub method: String,
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: String,
    pub duration: Duration,
    pub size: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_response() {
        let resp = HttpResponse {
            url: "https://example.com".to_string(),
            method: "GET".to_string(),
            status: 200,
            headers: vec![],
            body: "OK".to_string(),
            duration: Duration::from_millis(150),
            size: 2,
        };
        assert_eq!(resp.status, 200);
        assert_eq!(resp.size, 2);
        assert_eq!(resp.duration, Duration::from_millis(150));
    }

    #[test]
    fn response_clone() {
        let resp = HttpResponse {
            url: "https://example.com".to_string(),
            method: "POST".to_string(),
            status: 201,
            headers: vec![("Location".to_string(), "/resource/1".to_string())],
            body: r#"{"id": 1}"#.to_string(),
            duration: Duration::from_millis(200),
            size: 13,
        };
        let cloned = resp.clone();
        assert_eq!(resp.status, cloned.status);
        assert_eq!(resp.body, cloned.body);
        assert_eq!(resp.headers, cloned.headers);
    }

    #[test]
    fn response_status_codes() {
        let statuses = [200, 201, 301, 400, 404, 500, 503];
        for status in statuses {
            let resp = HttpResponse {
                url: String::new(),
                method: String::new(),
                status,
                headers: vec![],
                body: String::new(),
                duration: Duration::ZERO,
                size: 0,
            };
            assert_eq!(resp.status, status);
        }
    }
}
