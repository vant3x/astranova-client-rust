use crate::protocols::websocket::{WsMessage, WsMessageType, WsSender, WsStatus};

use iced::{
    widget::{button, column, container, row, scrollable, text, text_input},
    Alignment, Color, Element, Length, Renderer, Theme,
};
use iced_fonts::lucide;

#[derive(Debug, Clone)]
#[allow(dead_code)]
#[allow(clippy::enum_variant_names)]
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
    ReconnectDelayChanged(String),
    MaxRetriesChanged(String),
    SearchChanged(String),
    SubprotocolChanged(String),
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
    pub reconnect_delay_ms: u64,
    pub max_retries: u32,
    pub current_retries: u32,
    pub ws_sender: Option<WsSender>,
    pub search_query: String,
    pub subprotocol: String,
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
            reconnect_delay_ms: self.reconnect_delay_ms,
            max_retries: self.max_retries,
            current_retries: self.current_retries,
            ws_sender: self.ws_sender.clone(),
            search_query: self.search_query.clone(),
            subprotocol: self.subprotocol.clone(),
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
            reconnect_delay_ms: 3000,
            max_retries: 5,
            current_retries: 0,
            ws_sender: None,
            search_query: String::new(),
            subprotocol: String::new(),
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
                button(row![lucide::plug().size(14), text(" Connect")].spacing(4))
                    .on_press(Message::Connect)
            }
            WsStatus::Connecting => {
                button(row![lucide::loader().size(14), text(" Connecting...")].spacing(4))
            }
            WsStatus::Connected => {
                button(row![lucide::plug_zap().size(14), text(" Disconnect")].spacing(4))
                    .on_press(Message::Disconnect)
            }
        };

        let url_row = row![
            text_input("wss://echo.websocket.org", &self.url)
                .on_input(Message::UrlChanged)
                .padding(8),
            text_input("Subprotocol", &self.subprotocol)
                .on_input(Message::SubprotocolChanged)
                .padding(8)
                .width(Length::Fixed(150.0)),
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

        let reconnect_config = if self.auto_reconnect {
            let delay_input = text_input("Delay (ms)", &self.reconnect_delay_ms.to_string())
                .on_input(Message::ReconnectDelayChanged)
                .padding(5)
                .width(Length::Fixed(100.0));

            let max_retries_input = text_input("Max retries", &self.max_retries.to_string())
                .on_input(Message::MaxRetriesChanged)
                .padding(5)
                .width(Length::Fixed(80.0));

            let retry_info = if self.current_retries > 0 {
                text(format!(
                    "Retry {}/{}",
                    self.current_retries, self.max_retries
                ))
                .size(11)
                .color(Color::from_rgb(0.8, 0.7, 0.1))
            } else {
                text("").size(11)
            };

            row![
                text("Delay:").size(12),
                delay_input,
                text("Max:").size(12),
                max_retries_input,
                retry_info,
            ]
            .spacing(6)
            .align_y(Alignment::Center)
        } else {
            row![]
        };

        let header_toggle = button(
            row![
                if self.show_headers {
                    lucide::panel_left_close().size(14)
                } else {
                    lucide::panel_left_open().size(14)
                },
                text(if self.show_headers {
                    " Hide Headers"
                } else {
                    " Show Headers"
                })
                .size(12),
            ]
            .spacing(4),
        )
        .on_press(Message::ToggleHeaders);

        let headers_section = if self.show_headers {
            let mut header_list = column![].spacing(4);
            for (i, (k, v)) in self.headers.iter().enumerate() {
                header_list = header_list.push(
                    row![
                        text(format!("{}: {}", k, v)).size(12),
                        button(lucide::x().size(11)).on_press(Message::RemoveHeader(i)),
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
                button(lucide::plus().size(14)).on_press(Message::AddHeader),
            ]
            .spacing(8);

            column![header_list, add_header_row].spacing(8)
        } else {
            column![]
        };

        let search_input = text_input("Search messages...", &self.search_query)
            .on_input(Message::SearchChanged)
            .padding(5);

        let filtered_messages: Vec<_> = if self.search_query.is_empty() {
            self.messages.clone()
        } else {
            let query = self.search_query.to_lowercase();
            self.messages
                .iter()
                .filter(|m| {
                    m.data.to_lowercase().contains(&query)
                        || m.direction.to_lowercase().contains(&query)
                        || format!("{:?}", m.message_type)
                            .to_lowercase()
                            .contains(&query)
                })
                .cloned()
                .collect()
        };

        let mut message_list = column![].spacing(4);
        for msg in &filtered_messages {
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

            let formatted = msg.formatted_data();
            let data_display: String = formatted.chars().take(200).collect();
            let truncated = if formatted.len() > 200 {
                format!("{}...", data_display)
            } else {
                data_display
            };

            let timestamp = msg.timestamp.clone();
            let time_display = if timestamp.len() >= 10 {
                format!("{}:{}", &timestamp[..timestamp.len()-4], &timestamp[timestamp.len()-2..])
            } else {
                timestamp.clone()
            };

            let dir_clone = msg.direction.clone();
            message_list = message_list.push(
                column![
                    row![
                        text(dir_clone).size(13).color(dir_color),
                        text(type_label)
                            .size(11)
                            .color(Color::from_rgb(0.5, 0.5, 0.5)),
                        text(truncated).size(13),
                    ]
                    .spacing(6),
                    text(format!("  {}", time_display))
                        .size(10)
                        .color(Color::from_rgb(0.4, 0.4, 0.4)),
                ]
                .spacing(2),
            );
        }

        let is_connected = matches!(self.status, WsStatus::Connected);

        let input_row = row![
            text_input("Message...", &self.input)
                .on_input(Message::InputChanged)
                .padding(8),
            if is_connected {
                button(lucide::send().size(14)).on_press(Message::SendMessage(self.input.clone()))
            } else {
                button(lucide::send().size(14))
            },
        ]
        .spacing(8)
        .align_y(Alignment::Center);

        let clear_button = if self.messages.is_empty() {
            button(row![lucide::trash().size(14), text(" Clear")].spacing(4))
        } else {
            button(row![lucide::trash().size(14), text(" Clear")].spacing(4))
                .on_press(Message::Disconnected("cleared".to_string()))
        };

        let header = column![
            row![
                text("WebSocket").size(16),
                auto_reconnect_toggle,
                header_toggle,
                clear_button,
            ]
            .spacing(10)
            .align_y(Alignment::Center),
            reconnect_config,
        ]
        .spacing(4);

        container(
            column![
                header,
                url_row,
                headers_section,
                search_input,
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
