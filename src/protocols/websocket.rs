use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio_tungstenite::{connect_async, tungstenite::Message};

pub const WS_MAX_MESSAGES: usize = 1000;

static WS_CONNECTION_COUNTER: AtomicU64 = AtomicU64::new(0);

pub fn next_ws_connection_id() -> u64 {
    WS_CONNECTION_COUNTER.fetch_add(1, Ordering::Relaxed)
}

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WsRequest {
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub subprotocol: Option<String>,
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
    pub fn send(&self, text: &str) -> Result<(), String> {
        self.tx
            .send(Message::Text(text.to_string()))
            .map_err(|e| format!("Send error: {}", e))
    }

    pub fn send_binary(&self, data: Vec<u8>) -> Result<(), String> {
        self.tx
            .send(Message::Binary(data))
            .map_err(|e| format!("Send binary error: {}", e))
    }
}

pub struct WsConnection {
    pub receiver: mpsc::UnboundedReceiver<WsEvent>,
    pub sender: WsSender,
    pub shutdown_tx: Option<mpsc::UnboundedSender<()>>,
    pub write_handle: JoinHandle<()>,
    pub read_handle: JoinHandle<()>,
}

impl WsConnection {
    #[allow(dead_code)]
    pub fn shutdown(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
        self.write_handle.abort();
        self.read_handle.abort();
    }
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum WsEvent {
    Message(WsMessage),
    Connected,
    Disconnected(String),
    Error(String),
}

pub async fn connect_ws(request: &WsRequest) -> Result<WsConnection, String> {
    let mut request_builder = http::Request::builder();

    for (key, value) in &request.headers {
        request_builder = request_builder.header(key, value);
    }

    if let Some(ref subprotocol) = request.subprotocol {
        if !subprotocol.is_empty() {
            request_builder = request_builder.header("Sec-WebSocket-Protocol", subprotocol);
        }
    }

    let request = request_builder
        .uri(&request.url)
        .body(())
        .map_err(|e| format!("Failed to build WebSocket request: {}", e))?;
    let (ws_stream, _response) = connect_async(request)
        .await
        .map_err(|e| format!("WebSocket connection failed: {}", e))?;

    let (mut write, mut read) = ws_stream.split();

    let (tx_out, mut rx_out) = mpsc::unbounded_channel::<Message>();
    let (tx_event, rx_event) = mpsc::unbounded_channel::<WsEvent>();
    let (shutdown_tx, mut shutdown_rx) = mpsc::unbounded_channel::<()>();

    let tx_event_for_write = tx_event.clone();
    let tx_event_for_connected = tx_event.clone();

    let write_handle: JoinHandle<()> = tokio::spawn(async move {
        loop {
            tokio::select! {
                msg = rx_out.recv() => {
                    match msg {
                        Some(msg) => {
                            if let Err(e) = write.send(msg).await {
                                let _ = tx_event_for_write.send(WsEvent::Error(format!("Send error: {}", e)));
                                break;
                            }
                        }
                        None => break,
                    }
                }
                _ = shutdown_rx.recv() => {
                    let _ = write.send(Message::Close(None)).await;
                    break;
                }
            }
        }
    });

    let read_handle: JoinHandle<()> = tokio::spawn(async move {
        while let Some(result) = read.next().await {
            match result {
                Ok(msg) => {
                    if let Some(ws_msg) = parse_ws_message(msg) {
                        if tx_event.send(WsEvent::Message(ws_msg)).is_err() {
                            break;
                        }
                    }
                }
                Err(e) => {
                    let _ = tx_event.send(WsEvent::Error(format!("Read error: {}", e)));
                    break;
                }
            }
        }
        let _ = tx_event.send(WsEvent::Disconnected("Connection closed".to_string()));
    });

    let _ = tx_event_for_connected.send(WsEvent::Connected);

    Ok(WsConnection {
        receiver: rx_event,
        sender: WsSender { tx: tx_out },
        shutdown_tx: Some(shutdown_tx),
        write_handle,
        read_handle,
    })
}

pub fn parse_ws_message(msg: Message) -> Option<WsMessage> {
    match msg {
        Message::Text(text) => Some(WsMessage::incoming(WsMessageType::Text, text)),
        Message::Binary(data) => {
            let hex = data.iter().map(|b| format!("{:02x}", b)).collect::<String>();
            let preview = if data.len() <= 32 {
                hex.clone()
            } else {
                format!("{}... ({} bytes)", &hex[..64], data.len())
            };
            Some(WsMessage::incoming(WsMessageType::Binary, preview))
        }
        Message::Ping(data) => Some(WsMessage::incoming(
            WsMessageType::Ping,
            format!("{:?}", data),
        )),
        Message::Pong(data) => Some(WsMessage::incoming(
            WsMessageType::Pong,
            format!("{:?}", data),
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
}
