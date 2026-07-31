use axum::extract::State as AxumState;
use axum::http::{HeaderMap, HeaderName, HeaderValue, Method, StatusCode};
use axum::{response::IntoResponse, Router};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::oneshot;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MockEndpoint {
    pub id: i32,
    pub mock_server_id: i32,
    pub method: String,
    pub path: String,
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Option<String>,
    pub delay_ms: u64,
    pub sort_order: i32,
}

impl MockEndpoint {
    pub fn matches(&self, req_method: &str, req_path: &str) -> bool {
        self.method.eq_ignore_ascii_case(req_method) && self.path == req_path
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MockServerConfig {
    pub id: i32,
    pub name: String,
    pub port: u16,
    pub host: String,
    pub enabled: bool,
    pub endpoints: Vec<MockEndpoint>,
}

impl std::fmt::Display for MockServerConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum MockServerStatus {
    #[default]
    Stopped,
    Starting,
    Running {
        actual_port: u16,
    },
    Error(String),
}

impl std::fmt::Display for MockServerStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MockServerStatus::Stopped => write!(f, "Stopped"),
            MockServerStatus::Starting => write!(f, "Starting..."),
            MockServerStatus::Running { actual_port } => write!(f, "Running on :{}", actual_port),
            MockServerStatus::Error(e) => write!(f, "Error: {}", e),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MockServerLog {
    pub mock_server_id: i32,
    pub timestamp: String,
    pub method: String,
    pub path: String,
    pub status: u16,
    pub matched: bool,
    pub response_time_ms: u64,
}

#[derive(Clone)]
struct MockServerState {
    server_id: i32,
    endpoints: Vec<MockEndpoint>,
    log_tx: tokio::sync::mpsc::UnboundedSender<MockServerLog>,
}

async fn mock_handler(
    AxumState(state): AxumState<MockServerState>,
    method: Method,
    uri: axum::http::Uri,
    _headers: HeaderMap,
    body: String,
) -> impl IntoResponse {
    let start = std::time::Instant::now();
    let req_method = method.as_str().to_uppercase();
    let req_path = uri.path().to_string();

    log::info!(
        "[Mock] {} {} (body: {} bytes)",
        req_method,
        req_path,
        body.len()
    );

    let matched = state
        .endpoints
        .iter()
        .find(|ep| ep.matches(&req_method, &req_path));

    let (status, resp_headers, resp_body, delay_ms) = match matched {
        Some(ep) => {
            log::info!(
                "[Mock] Matched endpoint: {} {} -> {} (delay: {}ms)",
                ep.method,
                ep.path,
                ep.status,
                ep.delay_ms
            );
            (
                ep.status,
                ep.headers.clone(),
                ep.body.clone().unwrap_or_default(),
                ep.delay_ms,
            )
        }
        None => {
            log::warn!(
                "[Mock] No match for {} {}, returning 404",
                req_method,
                req_path
            );
            let not_found_headers =
                vec![("content-type".to_string(), "application/json".to_string())];
            (
                404u16,
                not_found_headers,
                r#"{"error": "No mock endpoint configured for this route"}"#.to_string(),
                0u64,
            )
        }
    };

    if delay_ms > 0 {
        tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
    }

    let mut response_headers = HeaderMap::new();
    for (name, value) in &resp_headers {
        if let (Ok(hn), Ok(hv)) = (HeaderName::from_str(name), HeaderValue::from_str(value)) {
            response_headers.insert(hn, hv);
        }
    }

    if !response_headers.contains_key("content-type") {
        let ct =
            if resp_body.trim_start().starts_with('{') || resp_body.trim_start().starts_with('[') {
                "application/json"
            } else {
                "text/plain"
            };
        if let Ok(hv) = HeaderValue::from_str(ct) {
            response_headers.insert("content-type", hv);
        }
    }

    let elapsed = start.elapsed().as_millis() as u64;

    let log_entry = MockServerLog {
        mock_server_id: state.server_id,
        timestamp: crate::utils::timestamp_seconds(),
        method: req_method,
        path: req_path,
        status,
        matched: matched.is_some(),
        response_time_ms: elapsed,
    };
    let _ = state.log_tx.send(log_entry);

    let status_code = StatusCode::from_u16(status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
    (status_code, response_headers, resp_body)
}

pub struct MockServerHandle {
    pub shutdown_tx: Arc<tokio::sync::Mutex<Option<oneshot::Sender<()>>>>,
    pub join_handle: Arc<tokio::task::JoinHandle<()>>,
    pub log_rx: Arc<tokio::sync::Mutex<tokio::sync::mpsc::UnboundedReceiver<MockServerLog>>>,
}

impl Clone for MockServerHandle {
    fn clone(&self) -> Self {
        Self {
            shutdown_tx: Arc::clone(&self.shutdown_tx),
            join_handle: Arc::clone(&self.join_handle),
            log_rx: Arc::clone(&self.log_rx),
        }
    }
}

impl std::fmt::Debug for MockServerHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MockServerHandle")
            .field("join_handle", &format!("{:?}", self.join_handle.id()))
            .finish()
    }
}

pub async fn start_mock_server(
    config: &MockServerConfig,
) -> Result<(MockServerHandle, u16), String> {
    let (log_tx, log_rx) = tokio::sync::mpsc::unbounded_channel();

    let endpoints = config.endpoints.clone();
    let state = MockServerState {
        server_id: config.id,
        endpoints,
        log_tx,
    };

    let app = Router::new().fallback(mock_handler).with_state(state);

    let host = if config.host.is_empty() {
        "127.0.0.1"
    } else {
        &config.host
    };
    let addr = format!("{}:{}", host, config.port);

    let listener = TcpListener::bind(&addr)
        .await
        .map_err(|e| format!("Failed to bind to {}: {}", addr, e))?;

    let actual_port = listener.local_addr().map_err(|e| e.to_string())?.port();

    let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();

    let join_handle = tokio::spawn(async move {
        let server = axum::serve(listener, app).with_graceful_shutdown(async move {
            let _ = shutdown_rx.await;
        });
        if let Err(e) = server.await {
            log::error!("[Mock] Server error: {}", e);
        }
    });

    Ok((
        MockServerHandle {
            shutdown_tx: Arc::new(tokio::sync::Mutex::new(Some(shutdown_tx))),
            join_handle: Arc::new(join_handle),
            log_rx: Arc::new(tokio::sync::Mutex::new(log_rx)),
        },
        actual_port,
    ))
}

pub fn stop_mock_server(handle: MockServerHandle) {
    if let Ok(mut guard) = handle.shutdown_tx.try_lock() {
        if let Some(tx) = guard.take() {
            let _ = tx.send(());
        }
    }
    handle.join_handle.abort();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_endpoint_matches() {
        let ep = MockEndpoint {
            id: 1,
            mock_server_id: 1,
            method: "GET".to_string(),
            path: "/api/users".to_string(),
            status: 200,
            headers: vec![],
            body: Some(r#"{"users": []}"#.to_string()),
            delay_ms: 0,
            sort_order: 0,
        };

        assert!(ep.matches("get", "/api/users"));
        assert!(ep.matches("GET", "/api/users"));
        assert!(!ep.matches("POST", "/api/users"));
        assert!(!ep.matches("GET", "/api/users/1"));
    }

    #[test]
    fn test_endpoint_matches_wildcard_path() {
        let ep = MockEndpoint {
            id: 1,
            mock_server_id: 1,
            method: "GET".to_string(),
            path: "/".to_string(),
            status: 200,
            headers: vec![],
            body: None,
            delay_ms: 0,
            sort_order: 0,
        };

        assert!(ep.matches("GET", "/"));
        assert!(!ep.matches("GET", "/anything"));
    }
}
