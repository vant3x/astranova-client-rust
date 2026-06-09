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

    match req.auth_type.as_str() {
        "bearer" => {
            if let Some(token) = &req.auth_data {
                view.auth = crate::data::auth::Auth::BearerToken(token.clone());
            }
        }
        "basic" => {
            if let Some(data) = &req.auth_data {
                let parts: Vec<&str> = data.splitn(2, ':').collect();
                if parts.len() == 2 {
                    view.auth = crate::data::auth::Auth::Basic {
                        user: parts[0].to_string(),
                        pass: parts[1].to_string(),
                    };
                }
            }
        }
        _ => {}
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

    view.params_editor.entries = request
        .config
        .proxy_url
        .iter()
        .enumerate()
        .map(|(i, p)| KeyValueEntry {
            id: i,
            key: "proxy".to_string(),
            value: p.clone(),
        })
        .collect();

    if !request.multipart_fields.is_empty() {
        view.body_type = BodyType::Multipart;
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
}
