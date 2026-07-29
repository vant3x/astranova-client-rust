use crate::protocols::websocket::{connect_ws, WsRequest, WsStats, WsStatus};
use crate::ui::app::{AstraioApp, Message};
use crate::ui::views::websocket_view;
use iced::Task;
use std::sync::{Arc, Mutex};

pub fn handle_message(app: &mut AstraioApp, message: websocket_view::Message) -> Task<Message> {
    match message {
        websocket_view::Message::UrlChanged(url) => {
            app.websocket_view.url = url;
            Task::none()
        }
        websocket_view::Message::HeaderKeyChanged(key) => {
            app.websocket_view.header_key = key;
            Task::none()
        }
        websocket_view::Message::HeaderValueChanged(value) => {
            app.websocket_view.header_value = value;
            Task::none()
        }
        websocket_view::Message::AddHeader => {
            let key = app.websocket_view.header_key.clone();
            let value = app.websocket_view.header_value.clone();
            if !key.is_empty() {
                app.websocket_view.headers.push((key, value));
                app.websocket_view.header_key.clear();
                app.websocket_view.header_value.clear();
            }
            Task::none()
        }
        websocket_view::Message::RemoveHeader(index) => {
            if index < app.websocket_view.headers.len() {
                app.websocket_view.headers.remove(index);
            }
            Task::none()
        }
        websocket_view::Message::Connect => handle_connect(app),
        websocket_view::Message::Disconnect => handle_disconnect(app),
        websocket_view::Message::Disconnected(reason) => handle_disconnected(app, reason),
        websocket_view::Message::SendMessage(text) => {
            if let Some(sender) = &app.websocket_view.ws_sender {
                if !text.is_empty() {
                    let bytes = text.len() as u64;
                    let _ = sender.send(&text);
                    app.websocket_view.stats.messages_sent += 1;
                    app.websocket_view.stats.bytes_sent += bytes;
                    app.websocket_view.last_sent_message = text.clone();
                    app.websocket_view
                        .add_message(crate::protocols::websocket::WsMessage::outgoing(text));
                    app.websocket_view.input.clear();
                }
            }
            Task::none()
        }
        websocket_view::Message::SendBinary(hex) => {
            if let Some(sender) = &app.websocket_view.ws_sender {
                if !hex.is_empty() {
                    let bytes: Vec<u8> = hex
                        .split_whitespace()
                        .filter_map(|s| u8::from_str_radix(s, 16).ok())
                        .collect();
                    if !bytes.is_empty() {
                        let byte_len = bytes.len() as u64;
                        let _ = sender.send_binary(bytes.clone());
                        app.websocket_view.stats.messages_sent += 1;
                        app.websocket_view.stats.bytes_sent += byte_len;
                        let hex_display = bytes
                            .iter()
                            .map(|b| format!("{:02X}", b))
                            .collect::<Vec<_>>()
                            .join(" ");
                        app.websocket_view
                            .add_message(crate::protocols::websocket::WsMessage {
                                direction: ">".to_string(),
                                message_type: crate::protocols::websocket::WsMessageType::Binary,
                                data: hex_display,
                                timestamp: crate::utils::timestamp_seconds(),
                            });
                        app.websocket_view.hex_input.clear();
                    }
                }
            }
            Task::none()
        }
        websocket_view::Message::SendPing => {
            if let Some(sender) = &app.websocket_view.ws_sender {
                let ping_ts = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis()
                    .to_string();
                let _ = sender.send_ping(ping_ts.as_bytes().to_vec());
                app.websocket_view.stats.messages_sent += 1;
                app.websocket_view.add_message(
                    crate::protocols::websocket::WsMessage::outgoing_ping(ping_ts),
                );
            }
            Task::none()
        }
        websocket_view::Message::SendClose(reason) => {
            if let Some(sender) = &app.websocket_view.ws_sender {
                let close_reason = if reason.is_empty() {
                    "Goodbye".to_string()
                } else {
                    reason
                };
                let _ = sender.send_close(&close_reason);
                app.websocket_view
                    .add_message(crate::protocols::websocket::WsMessage {
                        direction: ">".to_string(),
                        message_type: crate::protocols::websocket::WsMessageType::Close,
                        data: format!("Close sent: {}", close_reason),
                        timestamp: crate::utils::timestamp_seconds(),
                    });
            }
            let _ = handle_disconnect(app);
            Task::none()
        }
        websocket_view::Message::InputChanged(input) => {
            app.websocket_view.input = input;
            Task::none()
        }
        websocket_view::Message::CloseReasonChanged(reason) => {
            app.websocket_view.close_reason = reason;
            Task::none()
        }
        websocket_view::Message::HexInputChanged(hex) => {
            app.websocket_view.hex_input = hex;
            Task::none()
        }
        websocket_view::Message::MessageTypeSelected(filter) => {
            app.websocket_view.message_type_filter = filter;
            Task::none()
        }
        websocket_view::Message::ToggleHeaders => {
            app.websocket_view.show_headers = !app.websocket_view.show_headers;
            Task::none()
        }
        websocket_view::Message::ToggleAutoReconnect => {
            app.websocket_view.auto_reconnect = !app.websocket_view.auto_reconnect;
            if app.websocket_view.auto_reconnect {
                log::info!("WebSocket auto-reconnect enabled");
            } else {
                log::info!("WebSocket auto-reconnect disabled");
            }
            Task::none()
        }
        websocket_view::Message::ReconnectDelayChanged(delay) => {
            if let Ok(d) = delay.parse::<u64>() {
                app.websocket_view.reconnect_delay_ms = d;
            }
            Task::none()
        }
        websocket_view::Message::MaxRetriesChanged(retries) => {
            if let Ok(r) = retries.parse::<u32>() {
                app.websocket_view.max_retries = r;
            }
            Task::none()
        }
        websocket_view::Message::SearchChanged(query) => {
            app.websocket_view.search_query = query;
            Task::none()
        }
        websocket_view::Message::SubprotocolChanged(sub) => {
            app.websocket_view.subprotocol = sub;
            Task::none()
        }
        websocket_view::Message::ClearMessages => {
            app.websocket_view.messages.clear();
            Task::none()
        }
        websocket_view::Message::ClearHistory => {
            app.websocket_view.messages.clear();
            Task::none()
        }
        websocket_view::Message::ConnectTimeoutChanged(timeout) => {
            if let Ok(t) = timeout.parse::<u64>() {
                app.websocket_view.config.connect_timeout_ms = t;
            }
            Task::none()
        }
        websocket_view::Message::PingIntervalChanged(interval) => {
            if let Ok(i) = interval.parse::<u64>() {
                app.websocket_view.config.ping_interval_ms = i;
            }
            Task::none()
        }
        websocket_view::Message::ToggleSkipVerify => {
            app.websocket_view.config.tls.skip_verify = !app.websocket_view.config.tls.skip_verify;
            Task::none()
        }
        websocket_view::Message::ToggleShowTls => {
            app.websocket_view.show_tls = !app.websocket_view.show_tls;
            Task::none()
        }
        websocket_view::Message::ToggleShowAdvanced => {
            app.websocket_view.show_advanced = !app.websocket_view.show_advanced;
            Task::none()
        }
        websocket_view::Message::ToggleMessageExpand(timestamp, direction) => {
            let key = (timestamp, direction);
            app.websocket_view.expanded_message =
                if app.websocket_view.expanded_message.as_ref() == Some(&key) {
                    None
                } else {
                    Some(key)
                };
            Task::none()
        }
        websocket_view::Message::CopyMessage(data) => {
            if let Ok(mut clipboard) = arboard::Clipboard::new() {
                let _ = clipboard.set_text(data);
            }
            Task::none()
        }
        websocket_view::Message::ReSendMessage(data) => {
            if let Some(sender) = &app.websocket_view.ws_sender {
                if !data.is_empty() && matches!(app.websocket_view.status, WsStatus::Connected) {
                    let bytes = data.len() as u64;
                    let _ = sender.send(&data);
                    app.websocket_view.stats.messages_sent += 1;
                    app.websocket_view.stats.bytes_sent += bytes;
                    app.websocket_view.last_sent_message = data.clone();
                    app.websocket_view
                        .add_message(crate::protocols::websocket::WsMessage::outgoing(data));
                }
            }
            Task::none()
        }
    }
}

fn handle_connect(app: &mut AstraioApp) -> Task<Message> {
    let url = app.websocket_view.url.clone();
    let headers = app.websocket_view.headers.clone();
    let subprotocol = if app.websocket_view.subprotocol.is_empty() {
        None
    } else {
        Some(app.websocket_view.subprotocol.clone())
    };
    let config = app.websocket_view.config.clone();

    if url.is_empty() {
        app.websocket_view.status = WsStatus::Error("URL is required".to_string());
        return Task::none();
    }

    app.websocket_view.status = WsStatus::Connecting;
    app.websocket_view.current_retries = 0;
    app.websocket_view.stats = WsStats::default();

    log::info!("Connecting to WebSocket: {}", url);

    Task::perform(
        async move {
            let request = WsRequest {
                url,
                headers,
                subprotocol,
                config,
            };
            connect_ws(&request).await
        },
        |result| match result {
            Ok(conn) => Message::WsConnected(
                conn.sender,
                Arc::new(Mutex::new(Some(conn.receiver))),
                conn.shutdown_tx,
                Arc::new(Mutex::new(Some(conn.write_handle))),
                Arc::new(Mutex::new(Some(conn.read_handle))),
                Arc::new(Mutex::new(conn.ping_handle)),
            ),
            Err(e) => {
                log::error!("WebSocket connection failed: {}", e);
                Message::WebSocketMsg(websocket_view::Message::Disconnected(e.to_string()))
            }
        },
    )
}

fn handle_disconnect(app: &mut AstraioApp) -> Task<Message> {
    log::info!("Disconnecting WebSocket");
    if let Some(shutdown) = app.ws_shutdown.take() {
        let _ = shutdown.send(());
    }

    // Abort all active handles to prevent memory leaks
    if let Some(write_handle) = app.ws_write_handle.take() {
        if let Some(handle) = write_handle.lock().ok().and_then(|mut h| h.take()) {
            handle.abort();
        }
    }
    if let Some(read_handle) = app.ws_read_handle.take() {
        if let Some(handle) = read_handle.lock().ok().and_then(|mut h| h.take()) {
            handle.abort();
        }
    }
    if let Some(ping_handle) = app.ws_ping_handle.take() {
        if let Some(handle) = ping_handle.lock().ok().and_then(|mut h| h.take()) {
            handle.abort();
        }
    }

    app.ws_sender = None;
    app.ws_receiver = None;
    app.websocket_view.ws_sender = None;
    app.websocket_view.status = WsStatus::Disconnected;
    app.websocket_view.current_retries = 0;
    Task::none()
}

fn handle_disconnected(app: &mut AstraioApp, reason: String) -> Task<Message> {
    if !app.websocket_view.messages.is_empty() {
        let url = app.websocket_view.url.clone();
        let messages = app.websocket_view.messages.clone();
        let duration_ms = app
            .websocket_view
            .stats
            .connected_at
            .map(|t| t.elapsed().as_millis() as u64);

        let request_data = serde_json::to_string(&serde_json::json!({
            "url": url,
            "headers": app.websocket_view.headers,
            "subprotocol": app.websocket_view.subprotocol,
            "config": app.websocket_view.config,
        }))
        .ok();

        let response_data = serde_json::to_string(&messages).ok();

        let connected = if app.websocket_view.status.is_connected() {
            1
        } else {
            0
        };

        let _ = crate::services::history_service::save_raw(
            &app.db_conn,
            "WS",
            &url,
            Some(connected),
            duration_ms,
            request_data.as_deref(),
            response_data.as_deref(),
        );
        let _ = crate::services::history_service::trim(
            &app.db_conn,
            crate::persistence::database::DEFAULT_HISTORY_LIMIT,
        );
        app.history_view.entries =
            crate::services::history_service::get_all(&app.db_conn, 200).unwrap_or_default();
    }

    app.ws_sender = None;
    app.ws_receiver = None;
    app.websocket_view.ws_sender = None;
    app.websocket_view.status = WsStatus::Disconnected;

    if app.websocket_view.auto_reconnect
        && app.websocket_view.current_retries < app.websocket_view.max_retries
    {
        app.websocket_view.current_retries += 1;
        let delay = app.websocket_view.reconnect_delay_ms;
        let retries = app.websocket_view.current_retries;
        let max = app.websocket_view.max_retries;

        log::info!("Auto-reconnect {}/{} in {}ms", retries, max, delay);

        app.websocket_view.status = WsStatus::Connecting;

        let url = app.websocket_view.url.clone();
        let headers = app.websocket_view.headers.clone();
        let subprotocol = if app.websocket_view.subprotocol.is_empty() {
            None
        } else {
            Some(app.websocket_view.subprotocol.clone())
        };
        let config = app.websocket_view.config.clone();

        return Task::perform(
            async move {
                tokio::time::sleep(tokio::time::Duration::from_millis(delay)).await;
                let request = WsRequest {
                    url,
                    headers,
                    subprotocol,
                    config,
                };
                connect_ws(&request).await
            },
            |result| match result {
                Ok(conn) => Message::WsConnected(
                    conn.sender,
                    Arc::new(Mutex::new(Some(conn.receiver))),
                    conn.shutdown_tx,
                    Arc::new(Mutex::new(Some(conn.write_handle))),
                    Arc::new(Mutex::new(Some(conn.read_handle))),
                    Arc::new(Mutex::new(conn.ping_handle)),
                ),
                Err(e) => {
                    log::error!("WebSocket reconnection failed: {}", e);
                    Message::WebSocketMsg(websocket_view::Message::Disconnected(e.to_string()))
                }
            },
        );
    }

    if !reason.is_empty() && reason != "Connection closed" {
        log::warn!("WebSocket disconnected: {}", reason);
    }

    Task::none()
}

pub fn handle_ws_event(
    app: &mut AstraioApp,
    event: crate::protocols::websocket::WsEvent,
) -> Task<Message> {
    match event {
        crate::protocols::websocket::WsEvent::Message(msg) => {
            let is_incoming = msg.direction == "<";
            let byte_len = msg.data.len() as u64;
            if is_incoming {
                app.websocket_view.stats.messages_received += 1;
                app.websocket_view.stats.bytes_received += byte_len;
            }
            app.websocket_view.add_message(msg);
            Task::none()
        }
        crate::protocols::websocket::WsEvent::Disconnected(reason) => {
            handle_disconnected(app, reason)
        }
        crate::protocols::websocket::WsEvent::Error(e) => {
            log::error!("WebSocket error: {}", e);
            app.websocket_view.status = WsStatus::Error(e.clone());

            if app.websocket_view.auto_reconnect
                && app.websocket_view.current_retries < app.websocket_view.max_retries
            {
                if let Some(shutdown) = app.ws_shutdown.take() {
                    let _ = shutdown.send(());
                }
                if let Some(h) = app.ws_write_handle.take() {
                    if let Some(handle) = h.lock().ok().and_then(|mut h| h.take()) {
                        handle.abort();
                    }
                }
                if let Some(h) = app.ws_read_handle.take() {
                    if let Some(handle) = h.lock().ok().and_then(|mut h| h.take()) {
                        handle.abort();
                    }
                }
                if let Some(h) = app.ws_ping_handle.take() {
                    if let Some(handle) = h.lock().ok().and_then(|mut h| h.take()) {
                        handle.abort();
                    }
                }
                app.ws_sender = None;
                app.ws_receiver = None;
                app.websocket_view.ws_sender = None;
                return handle_disconnected(app, e);
            }

            Task::none()
        }
    }
}

pub fn handle_ws_connected(
    app: &mut AstraioApp,
    sender: crate::protocols::websocket::WsSender,
    receiver: Arc<
        Mutex<Option<tokio::sync::mpsc::UnboundedReceiver<crate::protocols::websocket::WsEvent>>>,
    >,
    shutdown_tx: Option<tokio::sync::mpsc::UnboundedSender<()>>,
    write_handle: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
    read_handle: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
    ping_handle: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
) {
    // Abort existing handles before setting new ones to prevent memory leaks
    if let Some(old_write) = app.ws_write_handle.take() {
        if let Some(handle) = old_write.lock().ok().and_then(|mut h| h.take()) {
            handle.abort();
        }
    }
    if let Some(old_read) = app.ws_read_handle.take() {
        if let Some(handle) = old_read.lock().ok().and_then(|mut h| h.take()) {
            handle.abort();
        }
    }
    if let Some(old_ping) = app.ws_ping_handle.take() {
        if let Some(handle) = old_ping.lock().ok().and_then(|mut h| h.take()) {
            handle.abort();
        }
    }

    app.ws_sender = Some(sender.clone());
    app.websocket_view.ws_sender = Some(sender);
    app.ws_receiver = Some(receiver);
    app.ws_shutdown = shutdown_tx;
    app.ws_write_handle = Some(write_handle);
    app.ws_read_handle = Some(read_handle);
    app.ws_ping_handle = Some(ping_handle);
    app.websocket_view.status = WsStatus::Connected;
    app.websocket_view.current_retries = 0;
    app.websocket_view.stats.connected_at = Some(std::time::Instant::now());
    log::info!("WebSocket connection established");
}
