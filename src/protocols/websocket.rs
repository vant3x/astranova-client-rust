use crate::error::AppError;
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::time::Instant;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode;
use tokio_tungstenite::tungstenite::protocol::frame::CloseFrame;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::Connector;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum WsMessageType {
    Text,
    Binary,
    Ping,
    Pong,
    Close,
}

impl std::fmt::Display for WsMessageType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WsMessageType::Text => write!(f, "Text"),
            WsMessageType::Binary => write!(f, "Binary"),
            WsMessageType::Ping => write!(f, "Ping"),
            WsMessageType::Pong => write!(f, "Pong"),
            WsMessageType::Close => write!(f, "Close"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WsMessage {
    pub direction: String,
    pub message_type: WsMessageType,
    pub data: String,
    pub timestamp: String,
}

impl WsMessage {
    pub fn incoming(msg_type: WsMessageType, data: String) -> Self {
        Self {
            direction: "<".to_string(),
            message_type: msg_type,
            data,
            timestamp: crate::utils::timestamp_seconds(),
        }
    }

    pub fn outgoing(data: String) -> Self {
        Self {
            direction: ">".to_string(),
            message_type: WsMessageType::Text,
            data,
            timestamp: crate::utils::timestamp_seconds(),
        }
    }

    pub fn outgoing_ping(data: String) -> Self {
        Self {
            direction: ">".to_string(),
            message_type: WsMessageType::Ping,
            data,
            timestamp: crate::utils::timestamp_seconds(),
        }
    }

    pub fn formatted_data(&self) -> String {
        match self.message_type {
            WsMessageType::Text => {
                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&self.data) {
                    if let Ok(pretty) = serde_json::to_string_pretty(&parsed) {
                        return pretty;
                    }
                }
                self.data.clone()
            }
            WsMessageType::Binary => {
                let bytes: Vec<u8> = self
                    .data
                    .split(' ')
                    .filter_map(|s| u8::from_str_radix(s, 16).ok())
                    .collect();
                let hex_formatted: Vec<String> = bytes.iter().map(|b| format!("{b:02X}")).collect();
                let hex_display = hex_formatted
                    .chunks(16)
                    .map(|chunk| chunk.join(" "))
                    .collect::<Vec<_>>()
                    .join("\n");
                let as_utf8 = String::from_utf8_lossy(&bytes);
                let utf8_display = if bytes
                    .iter()
                    .all(|b| b.is_ascii_graphic() || b.is_ascii_whitespace())
                {
                    format!("\n\nUTF-8: {as_utf8}")
                } else {
                    String::new()
                };
                format!(
                    "Hex ({} bytes):\n{}{}",
                    bytes.len(),
                    hex_display,
                    utf8_display
                )
            }
            _ => self.data.clone(),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub enum WsStatus {
    #[default]
    Disconnected,
    Connecting,
    Connected,
    Error(String),
}

impl WsStatus {
    pub fn is_connected(&self) -> bool {
        matches!(self, WsStatus::Connected)
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WsTlsConfig {
    pub skip_verify: bool,
    #[serde(default)]
    pub ca_cert_pem: Option<String>,
    #[serde(default)]
    pub client_cert_pem: Option<String>,
    #[serde(default)]
    pub client_key_pem: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WsConfig {
    #[serde(default = "default_connect_timeout")]
    pub connect_timeout_ms: u64,
    #[serde(default = "default_ping_interval")]
    pub ping_interval_ms: u64,
    #[serde(default)]
    pub tls: WsTlsConfig,
}

fn default_connect_timeout() -> u64 {
    10000
}
fn default_ping_interval() -> u64 {
    30000
}

impl Default for WsConfig {
    fn default() -> Self {
        Self {
            connect_timeout_ms: default_connect_timeout(),
            ping_interval_ms: default_ping_interval(),
            tls: WsTlsConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct WsStats {
    pub messages_sent: u64,
    pub messages_received: u64,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub connected_at: Option<Instant>,
}

impl WsStats {
    pub fn duration(&self) -> Option<std::time::Duration> {
        self.connected_at.map(|t| t.elapsed())
    }

    pub fn format_duration(&self) -> String {
        match self.duration() {
            Some(d) => {
                let secs = d.as_secs();
                if secs < 60 {
                    format!("{secs}s")
                } else if secs < 3600 {
                    format!("{}m {}s", secs / 60, secs % 60)
                } else {
                    format!("{}h {}m", secs / 3600, (secs % 3600) / 60)
                }
            }
            None => "-".to_string(),
        }
    }

    pub fn format_bytes(bytes: u64) -> String {
        if bytes >= 1_048_576 {
            format!("{:.1} MB", bytes as f64 / 1_048_576.0)
        } else if bytes >= 1024 {
            format!("{:.1} KB", bytes as f64 / 1024.0)
        } else {
            format!("{bytes} B")
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WsRequest {
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub subprotocol: Option<String>,
    #[serde(default)]
    pub config: WsConfig,
}

pub struct WsSender {
    tx: mpsc::UnboundedSender<Message>,
}

impl Clone for WsSender {
    fn clone(&self) -> Self {
        Self {
            tx: self.tx.clone(),
        }
    }
}

impl std::fmt::Debug for WsSender {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "WsSender")
    }
}

impl WsSender {
    pub fn send(&self, text: &str) -> Result<(), AppError> {
        self.tx
            .send(Message::Text(text.to_string()))
            .map_err(|e| AppError::WebSocket(format!("Send error: {e}")))
    }

    pub fn send_binary(&self, data: Vec<u8>) -> Result<(), AppError> {
        self.tx
            .send(Message::Binary(data))
            .map_err(|e| AppError::WebSocket(format!("Send binary error: {e}")))
    }

    pub fn send_ping(&self, data: Vec<u8>) -> Result<(), AppError> {
        self.tx
            .send(Message::Ping(data))
            .map_err(|e| AppError::WebSocket(format!("Send ping error: {e}")))
    }

    pub fn send_close(&self, reason: &str) -> Result<(), AppError> {
        let frame = CloseFrame {
            code: CloseCode::Normal,
            reason: Cow::Owned(reason.to_string()),
        };
        self.tx
            .send(Message::Close(Some(frame)))
            .map_err(|e| AppError::WebSocket(format!("Send close error: {e}")))
    }
}

pub struct WsConnection {
    pub receiver: mpsc::UnboundedReceiver<WsEvent>,
    pub sender: WsSender,
    pub shutdown_tx: Option<mpsc::UnboundedSender<()>>,
    pub write_handle: JoinHandle<()>,
    pub read_handle: JoinHandle<()>,
    pub ping_handle: Option<JoinHandle<()>>,
}

#[derive(Debug, Clone)]
pub enum WsEvent {
    Message(WsMessage),
    Disconnected(String),
    Error(String),
}

fn build_tls_connector(config: &WsTlsConfig) -> Result<Connector, AppError> {
    let mut builder = native_tls::TlsConnector::builder();
    builder.danger_accept_invalid_certs(config.skip_verify);

    if let Some(ref ca_pem) = config.ca_cert_pem {
        let cert = native_tls::Certificate::from_pem(ca_pem.as_bytes())
            .map_err(|e| AppError::WebSocket(format!("Invalid CA certificate: {e}")))?;
        builder.add_root_certificate(cert);
    }

    if let (Some(ref cert_pem), Some(ref key_pem)) =
        (&config.client_cert_pem, &config.client_key_pem)
    {
        let combined = format!("{cert_pem}\n{key_pem}");
        let identity = native_tls::Identity::from_pkcs8(combined.as_bytes(), combined.as_bytes())
            .or_else(|_| native_tls::Identity::from_pkcs8(cert_pem.as_bytes(), key_pem.as_bytes()))
            .map_err(|e| AppError::WebSocket(format!("Invalid client certificate: {e}")))?;
        builder.identity(identity);
    }

    let connector = builder
        .build()
        .map_err(|e| AppError::WebSocket(format!("TLS build error: {e}")))?;
    Ok(Connector::NativeTls(connector))
}

pub async fn connect_ws(request: &WsRequest) -> Result<WsConnection, AppError> {
    let mut request_builder = http::Request::builder();

    for (key, value) in &request.headers {
        request_builder = request_builder.header(key, value);
    }

    if let Some(ref subprotocol) = request.subprotocol {
        if !subprotocol.is_empty() {
            request_builder = request_builder.header("Sec-WebSocket-Protocol", subprotocol);
        }
    }

    let url = request.url.clone();
    let http_request = request_builder
        .uri(&url)
        .body(())
        .map_err(|e| AppError::WebSocket(format!("Failed to build WebSocket request: {e}")))?;

    let connect_timeout = std::time::Duration::from_millis(request.config.connect_timeout_ms);
    let ping_interval = std::time::Duration::from_millis(request.config.ping_interval_ms);

    let tls_config = &request.config.tls;

    let connect_future = async {
        if tls_config.skip_verify
            || tls_config.ca_cert_pem.is_some()
            || tls_config.client_cert_pem.is_some()
        {
            let tls_connector = build_tls_connector(tls_config)?;
            tokio_tungstenite::connect_async_tls_with_config(
                http_request,
                None,
                false,
                Some(tls_connector),
            )
            .await
            .map_err(|e| AppError::WebSocket(format!("WebSocket connection failed: {e}")))
        } else {
            tokio_tungstenite::connect_async(http_request)
                .await
                .map_err(|e| AppError::WebSocket(format!("WebSocket connection failed: {e}")))
        }
    };

    let (ws_stream, _response) = tokio::time::timeout(connect_timeout, connect_future)
        .await
        .map_err(|_| {
            AppError::WebSocket(format!(
                "Connection timed out after {}ms",
                request.config.connect_timeout_ms
            ))
        })??;

    let (mut write, mut read) = ws_stream.split();

    let (tx_out, mut rx_out) = mpsc::unbounded_channel::<Message>();
    let tx_out_for_read = tx_out.clone();
    let (tx_event, rx_event) = mpsc::unbounded_channel::<WsEvent>();
    let (shutdown_tx, mut shutdown_rx) = mpsc::unbounded_channel::<()>();

    let tx_event_for_write = tx_event.clone();

    let write_handle: JoinHandle<()> = tokio::spawn(async move {
        loop {
            tokio::select! {
                msg = rx_out.recv() => {
                    match msg {
                        Some(msg) => {
                            if let Err(e) = write.send(msg).await {
                                let _ = tx_event_for_write.send(WsEvent::Error(format!("Send error: {e}")));
                                break;
                            }
                        }
                        None => break,
                    }
                }
                _ = shutdown_rx.recv() => {
                    let close_frame = CloseFrame {
                        code: CloseCode::Normal,
                        reason: Cow::Borrowed("Client disconnected"),
                    };
                    let _ = write.send(Message::Close(Some(close_frame))).await;
                    break;
                }
            }
        }
    });

    let tx_event_for_read = tx_event.clone();
    let read_handle: JoinHandle<()> = tokio::spawn(async move {
        while let Some(result) = read.next().await {
            match result {
                Ok(msg) => {
                    let is_ping = matches!(&msg, Message::Ping(_));
                    let ping_data = if let Message::Ping(data) = &msg {
                        data.clone()
                    } else {
                        vec![]
                    };

                    if let Some(ws_msg) = parse_ws_message(msg) {
                        if tx_event_for_read.send(WsEvent::Message(ws_msg)).is_err() {
                            break;
                        }
                    }

                    if is_ping {
                        // Send actual pong frame back to server
                        let _ = tx_out_for_read.send(Message::Pong(ping_data));
                        // Also add to UI
                        let _ = tx_event_for_read.send(WsEvent::Message(WsMessage::incoming(
                            WsMessageType::Pong,
                            "auto-reply".to_string(),
                        )));
                    }
                }
                Err(e) => {
                    let _ = tx_event_for_read.send(WsEvent::Error(format!("Read error: {e}")));
                    break;
                }
            }
        }
        let _ = tx_event_for_read.send(WsEvent::Disconnected("Connection closed".to_string()));
    });

    let ping_handle = if ping_interval.as_millis() > 0 {
        let ping_tx = tx_event.clone();
        let ping_rx_out = tx_out.clone();
        Some(tokio::spawn(async move {
            loop {
                tokio::select! {
                    () = tokio::time::sleep(ping_interval) => {
                        let ping_data = b"ping".to_vec();
                        if ping_tx.send(WsEvent::Message(WsMessage::outgoing_ping(
                            "auto-ping".to_string(),
                        ))).is_err() {
                            break;
                        }
                        if ping_rx_out.send(Message::Ping(ping_data)).is_err() {
                            break;
                        }
                    }
                }
            }
        }))
    } else {
        None
    };

    Ok(WsConnection {
        receiver: rx_event,
        sender: WsSender { tx: tx_out },
        shutdown_tx: Some(shutdown_tx),
        write_handle,
        read_handle,
        ping_handle,
    })
}

pub fn parse_ws_message(msg: Message) -> Option<WsMessage> {
    match msg {
        Message::Text(text) => Some(WsMessage::incoming(WsMessageType::Text, text)),
        Message::Binary(data) => {
            let hex_display = data
                .iter()
                .map(|b| format!("{b:02X}"))
                .collect::<Vec<_>>()
                .join(" ");
            Some(WsMessage::incoming(WsMessageType::Binary, hex_display))
        }
        Message::Ping(data) => Some(WsMessage::incoming(
            WsMessageType::Ping,
            format!("{} bytes", data.len()),
        )),
        Message::Pong(data) => Some(WsMessage::incoming(
            WsMessageType::Pong,
            format!("{} bytes", data.len()),
        )),
        Message::Close(_) => Some(WsMessage::incoming(
            WsMessageType::Close,
            "closed".to_string(),
        )),
        Message::Frame(_) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ws_message_type_display() {
        assert_eq!(WsMessageType::Text.to_string(), "Text");
        assert_eq!(WsMessageType::Binary.to_string(), "Binary");
        assert_eq!(WsMessageType::Ping.to_string(), "Ping");
        assert_eq!(WsMessageType::Pong.to_string(), "Pong");
        assert_eq!(WsMessageType::Close.to_string(), "Close");
    }

    #[test]
    fn ws_message_incoming() {
        let msg = WsMessage::incoming(WsMessageType::Text, "hello".to_string());
        assert_eq!(msg.direction, "<");
        assert_eq!(msg.message_type, WsMessageType::Text);
        assert_eq!(msg.data, "hello");
    }

    #[test]
    fn ws_message_outgoing() {
        let msg = WsMessage::outgoing("world".to_string());
        assert_eq!(msg.direction, ">");
        assert_eq!(msg.message_type, WsMessageType::Text);
        assert_eq!(msg.data, "world");
    }

    #[test]
    fn ws_status_variants() {
        let s1 = WsStatus::Disconnected;
        let s2 = WsStatus::Connecting;
        let s3 = WsStatus::Connected;
        let s4 = WsStatus::Error("test".to_string());
        assert!(matches!(s1, WsStatus::Disconnected));
        assert!(matches!(s2, WsStatus::Connecting));
        assert!(matches!(s3, WsStatus::Connected));
        assert!(matches!(s4, WsStatus::Error(_)));
    }

    #[test]
    fn ws_request_clone() {
        let req = WsRequest {
            url: "wss://echo.websocket.org".to_string(),
            headers: vec![("Authorization".to_string(), "Bearer token".to_string())],
            subprotocol: None,
            config: WsConfig::default(),
        };
        let cloned = req.clone();
        assert_eq!(req.url, cloned.url);
        assert_eq!(req.headers, cloned.headers);
        assert_eq!(req.subprotocol, cloned.subprotocol);
    }

    #[test]
    fn ws_event_clone() {
        let msg = WsMessage::incoming(WsMessageType::Text, "test".to_string());
        let event = WsEvent::Message(msg.clone());
        match event {
            WsEvent::Message(m) => {
                assert_eq!(m.data, "test");
            }
            _ => panic!("Expected Message"),
        }
    }

    #[test]
    fn ws_config_default() {
        let config = WsConfig::default();
        assert_eq!(config.connect_timeout_ms, 10000);
        assert_eq!(config.ping_interval_ms, 30000);
        assert!(!config.tls.skip_verify);
        assert!(config.tls.ca_cert_pem.is_none());
        assert!(config.tls.client_cert_pem.is_none());
        assert!(config.tls.client_key_pem.is_none());
    }

    #[test]
    fn ws_stats_format_bytes() {
        assert_eq!(WsStats::format_bytes(0), "0 B");
        assert_eq!(WsStats::format_bytes(512), "512 B");
        assert_eq!(WsStats::format_bytes(1024), "1.0 KB");
        assert_eq!(WsStats::format_bytes(1536), "1.5 KB");
        assert_eq!(WsStats::format_bytes(1_048_576), "1.0 MB");
        assert_eq!(WsStats::format_bytes(2_621_440), "2.5 MB");
    }

    #[test]
    fn ws_stats_format_duration() {
        let stats = WsStats::default();
        assert_eq!(stats.format_duration(), "-");

        let stats = WsStats {
            connected_at: Some(
                Instant::now()
                    .checked_sub(std::time::Duration::from_secs(45))
                    .unwrap(),
            ),
            ..Default::default()
        };
        assert_eq!(stats.format_duration(), "45s");

        let stats = WsStats {
            connected_at: Some(
                Instant::now()
                    .checked_sub(std::time::Duration::from_secs(125))
                    .unwrap(),
            ),
            ..Default::default()
        };
        assert_eq!(stats.format_duration(), "2m 5s");
    }

    #[test]
    fn ws_tls_config_default() {
        let tls = WsTlsConfig::default();
        assert!(!tls.skip_verify);
        assert!(tls.ca_cert_pem.is_none());
        assert!(tls.client_cert_pem.is_none());
        assert!(tls.client_key_pem.is_none());
    }
}
