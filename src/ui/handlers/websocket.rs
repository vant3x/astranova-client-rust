use crate::protocols::websocket::{WsEvent, WsSender, WS_MAX_MESSAGES};
use crate::ui::app::{AstraNovaApp, Message};
use crate::ui::views::websocket_view;
use iced::Task;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;

pub fn handle_ws_event(app: &mut AstraNovaApp, event: WsEvent) -> Task<Message> {
    match event {
        WsEvent::Connected => {
            app.websocket_view.status = crate::protocols::websocket::WsStatus::Connected;
        }
        WsEvent::Message(msg) => {
            app.websocket_view.messages.push(msg);
            if app.websocket_view.messages.len() > WS_MAX_MESSAGES {
                let drain_count = app.websocket_view.messages.len() - WS_MAX_MESSAGES;
                app.websocket_view.messages.drain(..drain_count);
            }
        }
        WsEvent::Disconnected(reason) => {
            app.websocket_view.status = crate::protocols::websocket::WsStatus::Disconnected;
            app.ws_sender = None;
            log::info!("WebSocket disconnected: {}", reason);
        }
        WsEvent::Error(e) => {
            app.websocket_view.status = crate::protocols::websocket::WsStatus::Error(e.clone());
            app.ws_sender = None;
            log::error!("WebSocket error: {}", e);
        }
    }
    Task::none()
}

pub fn handle_connect(app: &mut AstraNovaApp) -> Task<Message> {
    let url = app.websocket_view.url.clone();
    let headers = app.websocket_view.headers.clone();
    let subprotocol = if app.websocket_view.subprotocol.is_empty() {
        None
    } else {
        Some(app.websocket_view.subprotocol.clone())
    };
    app.websocket_view.status = crate::protocols::websocket::WsStatus::Connecting;

    Task::perform(
        async move {
            let request = crate::protocols::websocket::WsRequest {
                url,
                headers,
                subprotocol,
            };
            crate::protocols::websocket::connect_ws(&request).await
        },
        |result| match result {
            Ok(conn) => Message::WsConnected(
                conn.sender,
                Arc::new(Mutex::new(Some(conn.receiver))),
                conn.shutdown_tx,
                Arc::new(Mutex::new(Some(conn.write_handle))),
                Arc::new(Mutex::new(Some(conn.read_handle))),
            ),
            Err(e) => Message::WebSocketMsg(websocket_view::Message::Disconnected(e)),
        },
    )
}

pub fn handle_disconnect(app: &mut AstraNovaApp) -> Task<Message> {
    if let Some(shutdown_tx) = app.ws_shutdown.take() {
        let _ = shutdown_tx.send(());
    }
    if let Some(handle_arc) = app.ws_write_handle.take() {
        if let Ok(mut guard) = handle_arc.lock() {
            if let Some(handle) = guard.take() {
                handle.abort();
            }
        }
    }
    if let Some(handle_arc) = app.ws_read_handle.take() {
        if let Ok(mut guard) = handle_arc.lock() {
            if let Some(handle) = guard.take() {
                handle.abort();
            }
        }
    }
    app.ws_sender = None;
    app.ws_receiver = None;
    app.websocket_view.status = crate::protocols::websocket::WsStatus::Disconnected;
    Task::none()
}

pub fn handle_disconnected(app: &mut AstraNovaApp, reason: String) -> Task<Message> {
    if reason == "cleared" {
        app.websocket_view.messages.clear();
        return Task::none();
    }

    app.ws_sender = None;
    app.ws_receiver = None;

    if app.websocket_view.auto_reconnect
        && app.websocket_view.current_retries < app.websocket_view.max_retries
    {
        app.websocket_view.current_retries += 1;
        app.websocket_view.status = crate::protocols::websocket::WsStatus::Connecting;

        let url = app.websocket_view.url.clone();
        let headers = app.websocket_view.headers.clone();
        let subprotocol = if app.websocket_view.subprotocol.is_empty() {
            None
        } else {
            Some(app.websocket_view.subprotocol.clone())
        };
        let delay = app.websocket_view.reconnect_delay_ms;

        log::info!(
            "Auto-reconnect attempt {}/{} after {}ms",
            app.websocket_view.current_retries,
            app.websocket_view.max_retries,
            delay
        );

        Task::perform(
            async move {
                tokio::time::sleep(tokio::time::Duration::from_millis(delay)).await;
                let request = crate::protocols::websocket::WsRequest {
                    url,
                    headers,
                    subprotocol,
                };
                crate::protocols::websocket::connect_ws(&request).await
            },
            |result| match result {
                Ok(conn) => Message::WsConnected(
                    conn.sender,
                    Arc::new(Mutex::new(Some(conn.receiver))),
                    conn.shutdown_tx,
                    Arc::new(Mutex::new(Some(conn.write_handle))),
                    Arc::new(Mutex::new(Some(conn.read_handle))),
                ),
                Err(e) => Message::WebSocketMsg(websocket_view::Message::Disconnected(e)),
            },
        )
    } else {
        app.websocket_view.status = crate::protocols::websocket::WsStatus::Disconnected;
        app.websocket_view.current_retries = 0;
        Task::none()
    }
}

pub fn handle_ws_connected(
    app: &mut AstraNovaApp,
    sender: WsSender,
    receiver_arc: Arc<Mutex<Option<mpsc::UnboundedReceiver<WsEvent>>>>,
    shutdown_tx: Option<mpsc::UnboundedSender<()>>,
    write_handle: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
    read_handle: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
) -> Task<Message> {
    app.ws_sender = Some(sender);
    app.ws_receiver = Some(receiver_arc);
    app.ws_shutdown = shutdown_tx;
    app.ws_write_handle = Some(write_handle);
    app.ws_read_handle = Some(read_handle);
    app.ws_connection_id = crate::protocols::websocket::next_ws_connection_id();
    app.websocket_view.status = crate::protocols::websocket::WsStatus::Connected;
    app.websocket_view.current_retries = 0;
    Task::none()
}

pub fn handle_message(app: &mut AstraNovaApp, msg: websocket_view::Message) -> Task<Message> {
    match msg {
        websocket_view::Message::Connect => handle_connect(app),
        websocket_view::Message::Disconnect => handle_disconnect(app),
        websocket_view::Message::Disconnected(reason) => handle_disconnected(app, reason),
        websocket_view::Message::ToggleHeaders => {
            app.websocket_view.show_headers = !app.websocket_view.show_headers;
            Task::none()
        }
        websocket_view::Message::ToggleAutoReconnect => {
            app.websocket_view.auto_reconnect = !app.websocket_view.auto_reconnect;
            if !app.websocket_view.auto_reconnect {
                app.websocket_view.current_retries = 0;
            }
            Task::none()
        }
        websocket_view::Message::ReconnectDelayChanged(delay) => {
            if let Ok(ms) = delay.parse::<u64>() {
                app.websocket_view.reconnect_delay_ms = ms;
            }
            Task::none()
        }
        websocket_view::Message::MaxRetriesChanged(retries) => {
            if let Ok(n) = retries.parse::<u32>() {
                app.websocket_view.max_retries = n;
            }
            Task::none()
        }
        websocket_view::Message::UrlChanged(url) => {
            app.websocket_view.url = url;
            Task::none()
        }
        websocket_view::Message::HeaderKeyChanged(key) => {
            app.websocket_view.header_key = key;
            Task::none()
        }
        websocket_view::Message::HeaderValueChanged(val) => {
            app.websocket_view.header_value = val;
            Task::none()
        }
        websocket_view::Message::AddHeader => {
            let key = app.websocket_view.header_key.clone();
            let val = app.websocket_view.header_value.clone();
            if !key.is_empty() {
                app.websocket_view.headers.push((key, val));
                app.websocket_view.header_key.clear();
                app.websocket_view.header_value.clear();
            }
            Task::none()
        }
        websocket_view::Message::RemoveHeader(idx) => {
            if idx < app.websocket_view.headers.len() {
                app.websocket_view.headers.remove(idx);
            }
            Task::none()
        }
        websocket_view::Message::InputChanged(input) => {
            app.websocket_view.input = input;
            Task::none()
        }
        websocket_view::Message::SearchChanged(query) => {
            app.websocket_view.search_query = query;
            Task::none()
        }
        websocket_view::Message::SubprotocolChanged(subprotocol) => {
            app.websocket_view.subprotocol = subprotocol;
            Task::none()
        }
        websocket_view::Message::SendMessage(text) if !text.is_empty() => {
            if let Some(sender) = &app.ws_sender {
                if sender.send(&text).is_ok() {
                    app.websocket_view.messages.push(
                        crate::protocols::websocket::WsMessage::outgoing(text.clone()),
                    );
                    if app.websocket_view.messages.len() > WS_MAX_MESSAGES {
                        let drain_count = app.websocket_view.messages.len() - WS_MAX_MESSAGES;
                        app.websocket_view.messages.drain(..drain_count);
                    }
                    app.websocket_view.input.clear();
                }
            }
            Task::none()
        }
        _ => Task::none(),
    }
}
