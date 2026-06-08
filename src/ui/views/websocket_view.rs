use crate::protocols::websocket::{WsMessage, WsMessageType, WsSender, WsStatus};

use iced::{
    widget::{button, column, container, row, scrollable, text, text_input},
    Alignment, Color, Element, Length, Renderer, Theme,
};

#[derive(Debug, Clone)]
pub enum Message {
    UrlChanged(String),
    HeaderKeyChanged(String),
    HeaderValueChanged(String),
    AddHeader,
    RemoveHeader(usize),
    Connect,
    ConnectedWithSender(WsSender),
    Disconnect,
    Disconnected(String),
    SendMessage(String),
    InputChanged(String),
    ToggleHeaders,
    ToggleAutoReconnect,
}

#[derive(Debug)]
pub struct WebSocketView {
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub header_key: String,
    pub header_value: String,
    pub status: WsStatus,
    pub messages: Vec<WsMessage>,
    pub input: String,
    pub show_headers: bool,
    pub auto_reconnect: bool,
    pub ws_sender: Option<WsSender>,
}

impl Clone for WebSocketView {
    fn clone(&self) -> Self {
        Self {
            url: self.url.clone(),
            headers: self.headers.clone(),
            header_key: self.header_key.clone(),
            header_value: self.header_value.clone(),
            status: self.status.clone(),
            messages: self.messages.clone(),
            input: self.input.clone(),
            show_headers: self.show_headers,
            auto_reconnect: self.auto_reconnect,
            ws_sender: self.ws_sender.clone(),
        }
    }
}

impl Default for WebSocketView {
    fn default() -> Self {
        Self {
            url: String::new(),
            headers: Vec::new(),
            header_key: String::new(),
            header_value: String::new(),
            status: WsStatus::Disconnected,
            messages: Vec::new(),
            input: String::new(),
            show_headers: false,
            auto_reconnect: false,
            ws_sender: None,
        }
    }
}

impl WebSocketView {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn view(&self) -> Element<'_, Message, Theme, Renderer> {
        let status_text = match &self.status {
            WsStatus::Disconnected => text("Disconnected").color(Color::from_rgb(0.5, 0.5, 0.5)),
            WsStatus::Connecting => text("Connecting...").color(Color::from_rgb(0.8, 0.7, 0.1)),
            WsStatus::Connected => text("Connected").color(Color::from_rgb(0.2, 0.7, 0.3)),
            WsStatus::Error(e) => {
                text(format!("Error: {}", e)).color(Color::from_rgb(0.8, 0.2, 0.2))
            }
        };

        let connect_button = match &self.status {
            WsStatus::Disconnected | WsStatus::Error(_) => {
                button("Connect").on_press(Message::Connect)
            }
            WsStatus::Connecting => button("Connecting..."),
            WsStatus::Connected => button("Disconnect").on_press(Message::Disconnect),
        };

        let url_row = row![
            text_input("wss://echo.websocket.org", &self.url)
                .on_input(Message::UrlChanged)
                .padding(8),
            connect_button,
            status_text.size(13),
        ]
        .spacing(8)
        .align_y(Alignment::Center);

        let auto_reconnect_label = if self.auto_reconnect {
            "[x] Auto-reconnect"
        } else {
            "[ ] Auto-reconnect"
        };
        let auto_reconnect_toggle =
            button(text(auto_reconnect_label).size(12)).on_press(Message::ToggleAutoReconnect);

        let header_toggle = button(
            text(if self.show_headers {
                "Hide Headers"
            } else {
                "Show Headers"
            })
            .size(12),
        )
        .on_press(Message::ToggleHeaders);

        let headers_section = if self.show_headers {
            let mut header_list = column![].spacing(4);
            for (i, (k, v)) in self.headers.iter().enumerate() {
                header_list = header_list.push(
                    row![
                        text(format!("{}: {}", k, v)).size(12),
                        button(text("x").size(11)).on_press(Message::RemoveHeader(i)),
                    ]
                    .spacing(8),
                );
            }

            let add_header_row = row![
                text_input("Key", &self.header_key)
                    .on_input(Message::HeaderKeyChanged)
                    .padding(5)
                    .width(Length::FillPortion(1)),
                text_input("Value", &self.header_value)
                    .on_input(Message::HeaderValueChanged)
                    .padding(5)
                    .width(Length::FillPortion(2)),
                button("+").on_press(Message::AddHeader),
            ]
            .spacing(8);

            column![header_list, add_header_row].spacing(8)
        } else {
            column![]
        };

        let mut message_list = column![].spacing(4);
        for msg in &self.messages {
            let dir_color = if msg.direction == ">" {
                Color::from_rgb(0.2, 0.4, 0.8)
            } else {
                Color::from_rgb(0.2, 0.7, 0.3)
            };

            let type_label = match msg.message_type {
                WsMessageType::Text => "TEXT",
                WsMessageType::Binary => "BIN",
                WsMessageType::Ping => "PING",
                WsMessageType::Pong => "PONG",
                WsMessageType::Close => "CLOSE",
            };

            let data_display: String = msg.data.chars().take(200).collect();
            let truncated = if msg.data.len() > 200 {
                format!("{}...", data_display)
            } else {
                data_display
            };

            message_list = message_list.push(
                row![
                    text(&msg.direction).size(13).color(dir_color),
                    text(type_label)
                        .size(11)
                        .color(Color::from_rgb(0.5, 0.5, 0.5)),
                    text(truncated).size(13),
                ]
                .spacing(6),
            );
        }

        let is_connected = matches!(self.status, WsStatus::Connected);

        let input_row = row![
            text_input("Message...", &self.input)
                .on_input(Message::InputChanged)
                .padding(8),
            if is_connected {
                button("Send").on_press(Message::SendMessage(self.input.clone()))
            } else {
                button("Send")
            },
        ]
        .spacing(8)
        .align_y(Alignment::Center);

        let clear_button = if self.messages.is_empty() {
            button("Clear")
        } else {
            button("Clear").on_press(Message::Disconnected("cleared".to_string()))
        };

        let header = row![
            text("WebSocket").size(16),
            auto_reconnect_toggle,
            header_toggle,
            clear_button,
        ]
        .spacing(10)
        .align_y(Alignment::Center);

        container(
            column![
                header,
                url_row,
                headers_section,
                scrollable(message_list).height(Length::Fill),
                input_row,
            ]
            .spacing(8)
            .padding(10),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }
}
