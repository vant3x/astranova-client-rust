use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio_tungstenite::{connect_async, tungstenite::Message};

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
            timestamp: chrono_now(),
        }
    }

    pub fn outgoing(data: String) -> Self {
        Self {
            direction: ">".to_string(),
            message_type: WsMessageType::Text,
            data,
            timestamp: chrono_now(),
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
}

pub async fn connect_ws(
    request: &WsRequest,
) -> Result<
    (
        futures_util::stream::SplitSink<
            tokio_tungstenite::WebSocketStream<
                tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
            >,
            Message,
        >,
        futures_util::stream::SplitStream<
            tokio_tungstenite::WebSocketStream<
                tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
            >,
        >,
    ),
    String,
> {
    let (ws_stream, _response) = connect_async(&request.url)
        .await
        .map_err(|e| format!("WebSocket connection failed: {}", e))?;

    Ok(ws_stream.split())
}

pub async fn send_text(
    sink: &mut futures_util::stream::SplitSink<
        tokio_tungstenite::WebSocketStream<
            tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
        >,
        Message,
    >,
    text: &str,
) -> Result<(), String> {
    sink.send(Message::Text(text.to_string()))
        .await
        .map_err(|e| format!("Send failed: {}", e))
}

pub fn parse_ws_message(msg: Message) -> Option<WsMessage> {
    match msg {
        Message::Text(text) => Some(WsMessage::incoming(WsMessageType::Text, text)),
        Message::Binary(data) => Some(WsMessage::incoming(
            WsMessageType::Binary,
            format!("{} bytes", data.len()),
        )),
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

fn chrono_now() -> String {
    use std::time::SystemTime;
    let duration = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default();
    format!("{}", duration.as_secs())
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
        };
        let cloned = req.clone();
        assert_eq!(req.url, cloned.url);
        assert_eq!(req.headers, cloned.headers);
    }
}
