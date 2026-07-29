use crate::protocols::websocket::{WsMessage, WsMessageType, WsSender, WsStats, WsStatus};

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
    Disconnect,
    Disconnected(String),
    SendMessage(String),
    SendBinary(String),
    SendPing,
    SendClose(String),
    InputChanged(String),
    HexInputChanged(String),
    CloseReasonChanged(String),
    MessageTypeSelected(MessageTypeFilter),
    ToggleHeaders,
    ToggleAutoReconnect,
    ReconnectDelayChanged(String),
    MaxRetriesChanged(String),
    SearchChanged(String),
    SubprotocolChanged(String),
    ClearMessages,
    ClearHistory,
    ConnectTimeoutChanged(String),
    PingIntervalChanged(String),
    ToggleSkipVerify,
    ToggleShowTls,
    ToggleShowAdvanced,
    ToggleMessageExpand(String, String),
    CopyMessage(String),
    ReSendMessage(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageTypeFilter {
    All,
    Text,
    Binary,
    Ping,
    Pong,
    Close,
}

impl std::fmt::Display for MessageTypeFilter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MessageTypeFilter::All => write!(f, "All"),
            MessageTypeFilter::Text => write!(f, "Text"),
            MessageTypeFilter::Binary => write!(f, "Binary"),
            MessageTypeFilter::Ping => write!(f, "Ping"),
            MessageTypeFilter::Pong => write!(f, "Pong"),
            MessageTypeFilter::Close => write!(f, "Close"),
        }
    }
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
    pub hex_input: String,
    pub close_reason: String,
    pub message_type_filter: MessageTypeFilter,
    pub show_headers: bool,
    pub auto_reconnect: bool,
    pub reconnect_delay_ms: u64,
    pub max_retries: u32,
    pub current_retries: u32,
    pub ws_sender: Option<WsSender>,
    pub search_query: String,
    pub subprotocol: String,
    pub config: crate::protocols::websocket::WsConfig,
    pub stats: WsStats,
    pub show_tls: bool,
    pub show_advanced: bool,
    pub max_messages: usize,
    pub expanded_message: Option<(String, String)>,
    pub last_sent_message: String,
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
            hex_input: self.hex_input.clone(),
            close_reason: self.close_reason.clone(),
            message_type_filter: self.message_type_filter,
            show_headers: self.show_headers,
            auto_reconnect: self.auto_reconnect,
            reconnect_delay_ms: self.reconnect_delay_ms,
            max_retries: self.max_retries,
            current_retries: self.current_retries,
            ws_sender: self.ws_sender.clone(),
            search_query: self.search_query.clone(),
            subprotocol: self.subprotocol.clone(),
            config: self.config.clone(),
            stats: self.stats.clone(),
            show_tls: self.show_tls,
            show_advanced: self.show_advanced,
            max_messages: self.max_messages,
            expanded_message: self.expanded_message.clone(),
            last_sent_message: self.last_sent_message.clone(),
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
            hex_input: String::new(),
            close_reason: String::new(),
            message_type_filter: MessageTypeFilter::All,
            show_headers: false,
            auto_reconnect: false,
            reconnect_delay_ms: 3000,
            max_retries: 5,
            current_retries: 0,
            ws_sender: None,
            search_query: String::new(),
            subprotocol: String::new(),
            config: crate::protocols::websocket::WsConfig::default(),
            stats: WsStats::default(),
            show_tls: false,
            show_advanced: false,
            max_messages: 10000,
            expanded_message: None,
            last_sent_message: String::new(),
        }
    }
}

impl WebSocketView {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_message(&mut self, msg: WsMessage) {
        self.messages.push(msg);
        if self.messages.len() > self.max_messages {
            let excess = self.messages.len() - self.max_messages;
            self.messages.drain(..excess);
        }
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

        let auto_reconnect_toggle = iced::widget::toggler(self.auto_reconnect)
            .label("Auto-reconnect")
            .on_toggle(|_checked| Message::ToggleAutoReconnect)
            .text_size(12);

        let advanced_toggle = iced::widget::toggler(self.show_advanced)
            .label("Advanced")
            .on_toggle(|_checked| Message::ToggleShowAdvanced)
            .text_size(12);

        let tls_toggle = iced::widget::toggler(self.show_tls)
            .label("TLS")
            .on_toggle(|_checked| Message::ToggleShowTls)
            .text_size(12);

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

        let advanced_section = if self.show_advanced {
            let timeout_input = text_input(
                "Connect timeout (ms)",
                &self.config.connect_timeout_ms.to_string(),
            )
            .on_input(Message::ConnectTimeoutChanged)
            .padding(5)
            .width(Length::Fixed(150.0));

            let ping_input = text_input(
                "Ping interval (ms)",
                &self.config.ping_interval_ms.to_string(),
            )
            .on_input(Message::PingIntervalChanged)
            .padding(5)
            .width(Length::Fixed(150.0));

            column![row![
                text("Connect timeout:").size(12),
                timeout_input,
                text("Ping interval:").size(12),
                ping_input,
            ]
            .spacing(6)
            .align_y(Alignment::Center),]
            .spacing(4)
        } else {
            column![]
        };

        let tls_section = if self.show_tls {
            let skip_verify_toggle = iced::widget::toggler(self.config.tls.skip_verify)
                .label("Skip certificate verification")
                .on_toggle(|_checked| Message::ToggleSkipVerify)
                .text_size(12);

            let status_info = match &self.status {
                WsStatus::Connected => {
                    let dur = self.stats.format_duration();
                    let sent = WsStats::format_bytes(self.stats.bytes_sent);
                    let recv = WsStats::format_bytes(self.stats.bytes_received);
                    text(format!(
                        "Duration: {} | Sent: {} | Recv: {} | Msgs: {}/{}",
                        dur, sent, recv, self.stats.messages_sent, self.stats.messages_received
                    ))
                    .size(11)
                    .color(Color::from_rgb(0.4, 0.4, 0.4))
                }
                _ => text("").size(11),
            };

            column![skip_verify_toggle, status_info,].spacing(4)
        } else {
            column![]
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

        let is_connected = matches!(self.status, WsStatus::Connected);
        let has_sender = self.ws_sender.is_some();
        let can_send = is_connected && has_sender && !self.input.is_empty();
        let can_send_binary = is_connected && has_sender && !self.hex_input.is_empty();

        let stats_row = if is_connected {
            let dur = self.stats.format_duration();
            let sent = WsStats::format_bytes(self.stats.bytes_sent);
            let recv = WsStats::format_bytes(self.stats.bytes_received);
            row![text(format!(
                "Connected {} | Sent: {} ({}) | Recv: {} ({})",
                dur, sent, self.stats.messages_sent, recv, self.stats.messages_received,
            ))
            .size(11)
            .color(Color::from_rgb(0.4, 0.4, 0.4)),]
        } else {
            row![]
        };

        let filter_buttons = row![
            self.filter_button(MessageTypeFilter::All),
            self.filter_button(MessageTypeFilter::Text),
            self.filter_button(MessageTypeFilter::Binary),
            self.filter_button(MessageTypeFilter::Ping),
            self.filter_button(MessageTypeFilter::Pong),
            self.filter_button(MessageTypeFilter::Close),
        ]
        .spacing(4);

        let search_row = row![
            text_input("Search...", &self.search_query)
                .on_input(Message::SearchChanged)
                .padding(5)
                .width(Length::Fill),
            filter_buttons,
        ]
        .spacing(8)
        .align_y(Alignment::Center);

        let filtered_messages: Vec<_> = self
            .messages
            .iter()
            .filter(|m| {
                let matches_type = self.message_type_filter == MessageTypeFilter::All
                    || matches!(
                        (&self.message_type_filter, &m.message_type),
                        (MessageTypeFilter::Text, WsMessageType::Text)
                            | (MessageTypeFilter::Binary, WsMessageType::Binary)
                            | (MessageTypeFilter::Ping, WsMessageType::Ping)
                            | (MessageTypeFilter::Pong, WsMessageType::Pong)
                            | (MessageTypeFilter::Close, WsMessageType::Close)
                    );
                let matches_search = self.search_query.is_empty()
                    || m.data
                        .to_lowercase()
                        .contains(&self.search_query.to_lowercase())
                    || m.direction
                        .to_lowercase()
                        .contains(&self.search_query.to_lowercase());
                matches_type && matches_search
            })
            .cloned()
            .collect();

        let total_messages = self.messages.len();
        let filtered_count = filtered_messages.len();

        let mut message_list = column![].spacing(4);
        if filtered_messages.is_empty() {
            let empty_text = if self.messages.is_empty() {
                match &self.status {
                    WsStatus::Disconnected | WsStatus::Error(_) => {
                        text("No messages yet. Enter a WebSocket URL and click Connect.")
                    }
                    WsStatus::Connecting => text("Connecting..."),
                    WsStatus::Connected => text("Connected. Send a message to begin."),
                }
            } else {
                text("No messages match the current filter.")
            };
            message_list = message_list.push(
                container(empty_text.size(14).color(Color::from_rgb(0.5, 0.5, 0.5)))
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .align_x(Alignment::Center)
                    .align_y(Alignment::Center),
            );
        } else {
            for msg in filtered_messages.iter() {
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

                let type_color = match msg.message_type {
                    WsMessageType::Text => Color::from_rgb(0.3, 0.7, 0.9),
                    WsMessageType::Binary => Color::from_rgb(0.8, 0.5, 0.1),
                    WsMessageType::Ping => Color::from_rgb(0.5, 0.5, 0.5),
                    WsMessageType::Pong => Color::from_rgb(0.5, 0.5, 0.5),
                    WsMessageType::Close => Color::from_rgb(0.8, 0.2, 0.2),
                };

                let formatted = msg.formatted_data();
                let is_expanded = self.expanded_message.as_ref()
                    == Some(&(msg.timestamp.clone(), msg.direction.clone()));
                let data_display = if is_expanded {
                    formatted.clone()
                } else {
                    let truncated: String = formatted.chars().take(200).collect();
                    if formatted.len() > 200 {
                        format!("{}...", truncated)
                    } else {
                        truncated
                    }
                };

                let byte_size = match msg.message_type {
                    WsMessageType::Binary => {
                        // msg.data is space-separated hex bytes, so count them
                        msg.data
                            .split_whitespace()
                            .filter(|s| !s.is_empty())
                            .count()
                    }
                    _ => msg.data.len(),
                };
                let size_label = if byte_size >= 1024 {
                    format!("{:.1}KB", byte_size as f64 / 1024.0)
                } else {
                    format!("{}B", byte_size)
                };

                let time_display = msg
                    .timestamp
                    .parse::<u64>()
                    .ok()
                    .and_then(|secs| {
                        let dt = std::time::UNIX_EPOCH + std::time::Duration::from_secs(secs);
                        Some(format!(
                            "{:02}:{:02}:{:02}",
                            (dt.duration_since(std::time::UNIX_EPOCH).ok()?.as_secs() / 3600) % 24,
                            (dt.duration_since(std::time::UNIX_EPOCH).ok()?.as_secs() / 60) % 60,
                            dt.duration_since(std::time::UNIX_EPOCH).ok()?.as_secs() % 60,
                        ))
                    })
                    .unwrap_or_else(|| msg.timestamp.clone());

                let dir_clone = msg.direction.clone();
                let expand_icon: Element<'_, Message, Theme, Renderer> = if is_expanded {
                    lucide::chevron_down().size(12).into()
                } else {
                    lucide::chevron_right().size(12).into()
                };
                let content = if is_expanded {
                    column![
                        row![
                            text(dir_clone).size(13).color(dir_color),
                            text(type_label).size(10).color(type_color),
                            text(data_display.clone()).size(12),
                        ]
                        .spacing(6),
                        row![text(format!("  {} - {}", time_display, size_label))
                            .size(10)
                            .color(Color::from_rgb(0.4, 0.4, 0.4)),],
                    ]
                    .spacing(2)
                } else {
                    column![
                        row![
                            text(dir_clone).size(13).color(dir_color),
                            text(type_label).size(10).color(type_color),
                            text(data_display).size(13),
                        ]
                        .spacing(6),
                        row![text(format!("  {} - {}", time_display, size_label))
                            .size(10)
                            .color(Color::from_rgb(0.4, 0.4, 0.4)),],
                    ]
                    .spacing(2)
                };
                let msg_data_clone = msg.data.clone();
                let copy_button = button(lucide::copy().size(10))
                    .on_press(Message::CopyMessage(msg_data_clone))
                    .style(button::secondary);

                let re_send_button = if msg.direction == ">" && msg.message_type == WsMessageType::Text {
                    let re_send_data = msg.data.clone();
                    button(lucide::repeat().size(10))
                        .on_press(Message::ReSendMessage(re_send_data))
                        .style(button::secondary)
                } else {
                    button(text(""))
                };

                let actions_row = row![copy_button, re_send_button].spacing(4);

                message_list = message_list.push(
                    button(
                        row![expand_icon, content, actions_row]
                            .spacing(6)
                            .align_y(Alignment::Start),
                    )
                    .on_press(Message::ToggleMessageExpand(
                        msg.timestamp.clone(),
                        msg.direction.clone(),
                    ))
                    .width(Length::Fill)
                    .style(
                        move |_: &Theme, _: iced::widget::button::Status| {
                            iced::widget::button::Style {
                                background: Some(iced::Color::TRANSPARENT.into()),
                                text_color: Color::WHITE,
                                border: iced::Border::default().rounded(4),
                                ..iced::widget::button::Style::default()
                            }
                        },
                    ),
                );
            }
        }

        let message_stats = if total_messages != filtered_count {
            text(format!(
                "Showing {}/{} messages",
                filtered_count, total_messages
            ))
            .size(11)
            .color(Color::from_rgb(0.5, 0.5, 0.5))
        } else if total_messages > 0 {
            text(format!("{} messages", total_messages))
                .size(11)
                .color(Color::from_rgb(0.5, 0.5, 0.5))
        } else {
            text("").size(11)
        };

        let input_row = row![
            text_input("Message...", &self.input)
                .on_input(Message::InputChanged)
                .padding(8)
                .width(Length::Fill),
            if can_send {
                button(row![lucide::send().size(14), text(" Send")].spacing(4))
                    .on_press(Message::SendMessage(self.input.clone()))
            } else {
                button(row![lucide::send().size(14), text(" Send")].spacing(4))
            },
        ]
        .spacing(8)
        .align_y(Alignment::Center);

        let hex_row = row![
            text_input("Hex: 48656c6c6f", &self.hex_input)
                .on_input(Message::HexInputChanged)
                .padding(5)
                .width(Length::Fill),
            if can_send_binary {
                button(row![lucide::binary().size(12), text(" Send Binary")].spacing(4))
                    .on_press(Message::SendBinary(self.hex_input.clone()))
            } else {
                button(row![lucide::binary().size(12), text(" Send Binary")].spacing(4))
            },
        ]
        .spacing(8)
        .align_y(Alignment::Center);

        let control_row = row![
            if is_connected {
                button(row![lucide::radio().size(12), text(" Ping")].spacing(4))
                    .on_press(Message::SendPing)
            } else {
                button(row![lucide::radio().size(12), text(" Ping")].spacing(4))
            },
            text_input("Close reason...", &self.close_reason)
                .on_input(Message::CloseReasonChanged)
                .padding(5)
                .width(Length::FillPortion(2)),
            if is_connected {
                button(row![lucide::octagon().size(12), text(" Close")].spacing(4))
                    .on_press(Message::SendClose(self.close_reason.clone()))
            } else {
                button(row![lucide::octagon().size(12), text(" Close")].spacing(4))
            },
        ]
        .spacing(8)
        .align_y(Alignment::Center);

        let clear_button = if self.messages.is_empty() {
            Element::from(row![])
        } else {
            button(row![lucide::trash().size(14), text(" Clear")].spacing(4))
                .on_press(Message::ClearMessages)
                .into()
        };

        let header = column![
            row![
                text("WebSocket").size(16),
                auto_reconnect_toggle,
                advanced_toggle,
                tls_toggle,
                header_toggle,
                clear_button,
            ]
            .spacing(10)
            .align_y(Alignment::Center),
            reconnect_config,
            advanced_section,
            tls_section,
        ]
        .spacing(4);

        let ws_warning: Element<'_, Message, Theme, Renderer> = {
            let is_ws = self.url.starts_with("ws://");
            if is_ws && !self.url.is_empty() {
                container(
                    row![
                        lucide::triangle_alert().size(14).color(Color::from_rgb(1.0, 0.8, 0.0)),
                        text("Warning: Using plaintext WebSocket (ws://). Use wss:// for secure connections.").size(12).color(Color::from_rgb(1.0, 0.8, 0.0)),
                    ]
                    .spacing(8)
                )
                .padding(8)
                .style(|_theme: &Theme| iced::widget::container::Style {
                    background: Some(iced::Color::from_rgba(1.0, 0.8, 0.0, 0.15).into()),
                    border: iced::Border::default()
                        .color(iced::Color::from_rgba(1.0, 0.8, 0.0, 0.4))
                        .width(1)
                        .rounded(4),
                    ..Default::default()
                })
                .into()
            } else {
                column![].into()
            }
        };

        container(
            column![
                header,
                url_row,
                ws_warning,
                headers_section,
                stats_row,
                search_row,
                message_stats,
                scrollable(message_list).height(Length::Fill),
                input_row,
                hex_row,
                control_row,
            ]
            .spacing(8)
            .padding(10),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }

    fn filter_button(&self, filter: MessageTypeFilter) -> Element<'_, Message, Theme, Renderer> {
        let is_active = self.message_type_filter == filter;
        let label = match filter {
            MessageTypeFilter::All => "All",
            MessageTypeFilter::Text => "T",
            MessageTypeFilter::Binary => "B",
            MessageTypeFilter::Ping => "P",
            MessageTypeFilter::Pong => "Po",
            MessageTypeFilter::Close => "C",
        };
        let btn = button(text(label).size(10)).on_press(Message::MessageTypeSelected(filter));
        if is_active {
            btn.style(button::secondary).into()
        } else {
            btn.into()
        }
    }
}
