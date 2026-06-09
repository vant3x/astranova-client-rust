use crate::http_client::request::HttpRequest;
use crate::http_client::response::HttpResponse;
use crate::persistence::database::{self, RequestHistoryEntry};
use rusqlite::Connection;

#[allow(dead_code)]
pub fn save(
    conn: &Connection,
    request: &HttpRequest,
    response: &HttpResponse,
) -> Result<(), String> {
    let request_data = serde_json::to_string(request).map_err(|e| e.to_string())?;
    let response_data = serde_json::to_string(response).map_err(|e| e.to_string())?;
    database::save_request_history(
        conn,
        &response.method,
        &response.url,
        Some(response.status),
        Some(response.duration.as_millis() as u64),
        Some(&request_data),
        Some(&response_data),
    )
    .map_err(|e| e.to_string())
}

pub fn save_raw(
    conn: &Connection,
    method: &str,
    url: &str,
    status: Option<u16>,
    duration_ms: Option<u64>,
    request_data: Option<&str>,
    response_data: Option<&str>,
) -> Result<(), String> {
    database::save_request_history(conn, method, url, status, duration_ms, request_data, response_data)
        .map_err(|e| e.to_string())
}

pub fn get_all(conn: &Connection, limit: usize) -> Vec<RequestHistoryEntry> {
    database::get_request_history(conn, limit).unwrap_or_default()
}

pub fn get_by_id(conn: &Connection, id: i32) -> Option<RequestHistoryEntry> {
    database::get_request_history_entry_by_id(conn, id)
        .ok()
        .flatten()
}

pub fn clear(conn: &Connection) {
    let _ = database::delete_request_history(conn);
}

#[allow(dead_code)]
pub fn restore_request(entry: &RequestHistoryEntry) -> Option<HttpRequest> {
    entry
        .request_data
        .as_ref()
        .and_then(|data| serde_json::from_str(data).ok())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::http_client::config::RequestConfig;
    use std::time::Duration;

    fn setup_test_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute(
            "CREATE TABLE IF NOT EXISTS request_history (
                id INTEGER PRIMARY KEY,
                method TEXT NOT NULL,
                url TEXT NOT NULL,
                status INTEGER,
                duration_ms INTEGER,
                timestamp TEXT NOT NULL,
                request_data TEXT,
                response_data TEXT
            )",
            [],
        )
        .unwrap();
        conn
    }

    fn make_request(method: &str, url: &str) -> HttpRequest {
        HttpRequest {
            method: method.to_string(),
            url: url.to_string(),
            headers: vec![],
            body: None,
            config: RequestConfig::default(),
            multipart_fields: vec![],
        }
    }

    fn make_response(method: &str, url: &str, status: u16) -> HttpResponse {
        HttpResponse {
            method: method.to_string(),
            url: url.to_string(),
            status,
            headers: vec![],
            body: "OK".to_string(),
            duration: Duration::from_millis(100),
            size: 2,
            redirect_chain: vec![],
        }
    }

    #[test]
    fn save_and_get_request() {
        let conn = setup_test_db();
        let req = make_request("GET", "https://example.com");
        let resp = make_response("GET", "https://example.com", 200);

        save(&conn, &req, &resp).unwrap();

        let entries = get_all(&conn, 10);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].method, "GET");
        assert!(entries[0].request_data.is_some());
        assert!(entries[0].response_data.is_some());
    }

    #[test]
    fn restore_request_from_entry() {
        let conn = setup_test_db();
        let mut req = make_request("POST", "https://api.example.com");
        req.headers
            .push(("Content-Type".to_string(), "application/json".to_string()));
        req.body = Some(r#"{"key":"value"}"#.to_string());
        let resp = make_response("POST", "https://api.example.com", 201);

        save(&conn, &req, &resp).unwrap();

        let entries = get_all(&conn, 10);
        let restored = restore_request(&entries[0]).unwrap();
        assert_eq!(restored.method, "POST");
        assert_eq!(restored.url, "https://api.example.com");
        assert_eq!(restored.headers.len(), 1);
        assert_eq!(restored.body, Some(r#"{"key":"value"}"#.to_string()));
    }

    #[test]
    fn clear_removes_all() {
        let conn = setup_test_db();
        let req = make_request("GET", "https://example.com");
        let resp = make_response("GET", "https://example.com", 200);
        save(&conn, &req, &resp).unwrap();

        clear(&conn);
        let entries = get_all(&conn, 10);
        assert!(entries.is_empty());
    }
}
