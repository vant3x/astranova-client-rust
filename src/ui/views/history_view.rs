use crate::persistence::database::RequestHistoryEntry;
use crate::ui::theme::{self, ThemeColors};
use iced::{
    widget::{button, column, container, row, scrollable, text, text_input},
    Alignment, Color, Element, Length, Padding, Renderer, Theme,
};
use iced_fonts::lucide;

#[derive(Debug, Clone)]
pub enum Message {
    ResendEntry(i32),
    RequestDeleteEntry(i32),
    ConfirmDeleteEntry(i32),
    CancelDeleteEntry,
    RequestClearHistory,
    ConfirmClearHistory,
    CancelClearHistory,
    SearchChanged(String),
    FilterMethod(String),
    ExportHistory,
    ViewResponse(i32),
    CloseResponse,
}

#[derive(Debug, Default)]
pub struct HistoryView {
    pub entries: Vec<RequestHistoryEntry>,
    pub selected_index: Option<usize>,
    pub search_query: String,
    pub filter_method: String,
    pub viewing_response: Option<RequestHistoryEntry>,
    pub pending_delete_entry: Option<i32>,
    pub pending_clear_history: bool,
}

impl Clone for HistoryView {
    fn clone(&self) -> Self {
        Self {
            entries: self.entries.clone(),
            selected_index: self.selected_index,
            search_query: self.search_query.clone(),
            filter_method: self.filter_method.clone(),
            viewing_response: self.viewing_response.clone(),
            pending_delete_entry: self.pending_delete_entry,
            pending_clear_history: self.pending_clear_history,
        }
    }
}

impl HistoryView {
    pub fn new() -> Self {
        Self::default()
    }

    fn filtered_entries(&self) -> Vec<&RequestHistoryEntry> {
        self.entries
            .iter()
            .filter(|e| {
                let matches_search = if self.search_query.is_empty() {
                    true
                } else {
                    let q = self.search_query.to_lowercase();
                    e.url.to_lowercase().contains(&q)
                        || e.method.to_lowercase().contains(&q)
                        || e.request_data
                            .as_ref()
                            .map(|d| d.to_lowercase().contains(&q))
                            .unwrap_or(false)
                        || e.response_data
                            .as_ref()
                            .map(|d| d.to_lowercase().contains(&q))
                            .unwrap_or(false)
                };
                let matches_method = if self.filter_method.is_empty() {
                    true
                } else {
                    e.method.eq_ignore_ascii_case(&self.filter_method)
                };
                matches_search && matches_method
            })
            .collect()
    }

    pub fn update(&mut self, message: Message) -> Option<i32> {
        match message {
            Message::ResendEntry(entry_id) => Some(entry_id),
            Message::RequestDeleteEntry(entry_id) => {
                self.pending_delete_entry = Some(entry_id);
                None
            }
            Message::ConfirmDeleteEntry(entry_id) => {
                self.pending_delete_entry = None;
                self.entries.retain(|e| e.id != entry_id);
                self.selected_index = None;
                if self
                    .viewing_response
                    .as_ref()
                    .map(|e| e.id == entry_id)
                    .unwrap_or(false)
                {
                    self.viewing_response = None;
                }
                None
            }
            Message::CancelDeleteEntry => {
                self.pending_delete_entry = None;
                None
            }
            Message::RequestClearHistory => {
                self.pending_clear_history = true;
                None
            }
            Message::ConfirmClearHistory => {
                self.pending_clear_history = false;
                self.entries.clear();
                self.selected_index = None;
                self.search_query.clear();
                self.filter_method.clear();
                self.viewing_response = None;
                None
            }
            Message::CancelClearHistory => {
                self.pending_clear_history = false;
                None
            }
            Message::SearchChanged(query) => {
                self.search_query = query;
                None
            }
            Message::FilterMethod(method) => {
                if self.filter_method == method {
                    self.filter_method.clear();
                } else {
                    self.filter_method = method;
                }
                None
            }
            Message::ExportHistory => None,
            Message::ViewResponse(entry_id) => {
                self.viewing_response = self.entries.iter().find(|e| e.id == entry_id).cloned();
                None
            }
            Message::CloseResponse => {
                self.viewing_response = None;
                None
            }
        }
    }

    fn build_response_panel(entry: &RequestHistoryEntry) -> Element<'_, Message, Theme, Renderer> {
        let status_color = theme::status_color(entry.status.unwrap_or(0));
        let status_text = match entry.status {
            Some(s) => format!("{} {}", s, theme::status_label(s)),
            None => "N/A".to_string(),
        };
        let duration_text = match entry.duration_ms {
            Some(d) => format!("{}ms", d),
            None => "N/A".to_string(),
        };

        let close_btn = button(
            row![lucide::x().size(12), text(" Close").size(11)].spacing(2),
        )
        .padding(Padding::from([4, 10]))
        .style(|_theme, _status| button::Style {
            background: Some(iced::Background::Color(ThemeColors::BG_LIGHT)),
            text_color: ThemeColors::TEXT_SECONDARY,
            border: iced::Border::default()
                .rounded(4)
                .color(ThemeColors::BORDER)
                .width(1),
            ..button::Style::default()
        })
        .on_press(Message::CloseResponse);

        let header = row![
            text("Response Details").size(14).color(status_color),
            text(format!("  {}  ", status_text))
                .size(12)
                .color(ThemeColors::TEXT_SECONDARY),
            text(duration_text)
                .size(11)
                .color(ThemeColors::TEXT_MUTED),
            close_btn,
        ]
        .spacing(8)
        .align_y(Alignment::Center);

        let mut response_content = column![header].spacing(8);

        if let Some(response_data) = &entry.response_data {
            if let Ok(resp) =
                serde_json::from_str::<crate::http_client::response::HttpResponse>(response_data)
            {
                let headers = resp.headers.clone();
                let body = resp.body.clone();
                let size = resp.size;

                if !headers.is_empty() {
                    let mut headers_col = column![].spacing(3);
                    for (k, v) in headers {
                        headers_col = headers_col.push(
                            row![
                                text(k.to_string())
                                    .size(11)
                                    .color(ThemeColors::ACCENT)
                                    .font(iced::Font {
                                        weight: iced::font::Weight::Medium,
                                        ..iced::font::Font::default()
                                    }),
                                text(": ").size(11).color(ThemeColors::TEXT_MUTED),
                                text(v).size(11).color(ThemeColors::TEXT_PRIMARY),
                            ]
                            .spacing(0),
                        );
                    }
                    response_content = response_content
                        .push(
                            text("Headers")
                                .size(12)
                                .color(ThemeColors::TEXT_SECONDARY),
                        )
                        .push(
                            container(scrollable(headers_col).height(Length::Fixed(140.0)))
                                .padding(8)
                                .style(|_theme: &Theme| iced::widget::container::Style {
                                    background: Some(iced::Background::Color(ThemeColors::BG_DARK)),
                                    border: iced::Border::default()
                                        .rounded(6)
                                        .color(ThemeColors::BORDER)
                                        .width(1),
                                    ..iced::widget::container::Style::default()
                                }),
                        );
                }

                let body_display: String = body.chars().take(2000).collect();
                let body_truncated = if body.len() > 2000 {
                    format!("{}...", body_display)
                } else {
                    body_display
                };
                let size_label = if size > 1024 {
                    format!("{:.1} KB", size as f64 / 1024.0)
                } else {
                    format!("{} B", size)
                };
                response_content = response_content
                    .push(
                        text(format!("Body ({})", size_label))
                            .size(12)
                            .color(ThemeColors::TEXT_SECONDARY),
                    )
                    .push(
                        container(scrollable(
                            text(body_truncated)
                                .size(11)
                                .font(iced::Font::MONOSPACE)
                                .color(ThemeColors::TEXT_PRIMARY),
                        ))
                        .height(Length::Fixed(200.0))
                        .padding(8)
                        .style(|_theme: &Theme| iced::widget::container::Style {
                            background: Some(iced::Background::Color(ThemeColors::BG_DARK)),
                            border: iced::Border::default()
                                .rounded(6)
                                .color(ThemeColors::BORDER)
                                .width(1),
                            ..iced::widget::container::Style::default()
                        }),
                    );
            } else {
                response_content = response_content
                    .push(
                        text("Response data (raw)")
                            .size(12)
                            .color(ThemeColors::TEXT_SECONDARY),
                    )
                    .push(
                        container(scrollable(text(response_data).size(11).font(iced::Font::MONOSPACE)))
                            .height(Length::Fixed(200.0))
                            .padding(8)
                            .style(|_theme: &Theme| iced::widget::container::Style {
                                background: Some(iced::Background::Color(ThemeColors::BG_DARK)),
                                border: iced::Border::default()
                                    .rounded(6)
                                    .color(ThemeColors::BORDER)
                                    .width(1),
                                ..iced::widget::container::Style::default()
                            }),
                    );
            }
        } else {
            response_content = response_content.push(
                text("No response data stored")
                    .size(12)
                    .color(ThemeColors::TEXT_MUTED),
            );
        }

        container(response_content)
            .padding(12)
            .style(|_theme: &Theme| iced::widget::container::Style {
                background: Some(iced::Background::Color(ThemeColors::BG_MEDIUM)),
                border: iced::Border::default()
                    .rounded(8)
                    .color(ThemeColors::BORDER)
                    .width(1),
                ..iced::widget::container::Style::default()
            })
            .width(Length::Fill)
            .into()
    }

    pub fn view(&self) -> Element<'_, Message, Theme, Renderer> {
        // ── Clear button with confirmation ──
        let clear_button: Element<'_, Message, Theme, Renderer> = if self.pending_clear_history {
            row![
                text("Clear all?")
                    .size(12)
                    .color(ThemeColors::ERROR),
                button(
                    row![lucide::check().size(11), text(" Yes").size(11)].spacing(2),
                )
                .padding(Padding::from([3, 8]))
                .style(|_theme, _status| button::Style {
                    background: Some(iced::Background::Color(ThemeColors::ERROR_DIM)),
                    text_color: ThemeColors::ERROR,
                    border: iced::Border::default()
                        .rounded(4)
                        .color(ThemeColors::ERROR)
                        .width(1),
                    ..button::Style::default()
                })
                .on_press(Message::ConfirmClearHistory),
                button(
                    row![lucide::x().size(11), text(" No").size(11)].spacing(2),
                )
                .padding(Padding::from([3, 8]))
                .style(|_theme, _status| button::Style {
                    background: Some(iced::Background::Color(ThemeColors::BG_LIGHT)),
                    text_color: ThemeColors::TEXT_SECONDARY,
                    border: iced::Border::default()
                        .rounded(4)
                        .color(ThemeColors::BORDER)
                        .width(1),
                    ..button::Style::default()
                })
                .on_press(Message::CancelClearHistory),
            ]
            .spacing(6)
            .align_y(Alignment::Center)
            .into()
        } else {
            button(
                row![lucide::trash().size(13), text(" Clear").size(12)].spacing(4),
            )
            .padding(Padding::from([5, 10]))
            .style(|_theme, _status| button::Style {
                background: Some(iced::Background::Color(ThemeColors::BG_LIGHT)),
                text_color: if self.entries.is_empty() {
                    ThemeColors::TEXT_DIM
                } else {
                    ThemeColors::TEXT_SECONDARY
                },
                border: iced::Border::default()
                    .rounded(4)
                    .color(ThemeColors::BORDER)
                    .width(1),
                ..button::Style::default()
            })
            .on_press_maybe(if self.entries.is_empty() {
                None
            } else {
                Some(Message::RequestClearHistory)
            })
            .into()
        };

        let export_button: Element<'_, Message, Theme, Renderer> = button(
            row![lucide::download().size(13), text(" Export").size(12)].spacing(4),
        )
        .padding(Padding::from([5, 10]))
        .style(|_theme, _status| button::Style {
            background: Some(iced::Background::Color(ThemeColors::BG_LIGHT)),
            text_color: if self.entries.is_empty() {
                ThemeColors::TEXT_DIM
            } else {
                ThemeColors::TEXT_SECONDARY
            },
            border: iced::Border::default()
                .rounded(4)
                .color(ThemeColors::BORDER)
                .width(1),
            ..button::Style::default()
        })
        .on_press_maybe(if self.entries.is_empty() {
            None
        } else {
            Some(Message::ExportHistory)
        })
        .into();

        // ── Header ──
        let header = row![
            text("History").size(16).color(ThemeColors::TEXT_PRIMARY),
            clear_button,
            export_button,
        ]
        .spacing(10)
        .align_y(Alignment::Center);

        // ── Search input ──
        let search_input = text_input("  Search by URL, method, body...", &self.search_query)
            .on_input(Message::SearchChanged)
            .padding(Padding::from([8, 10]))
            .width(Length::Fill)
            .style(|_theme: &Theme, status: iced::widget::text_input::Status| {
                let border_color = match status {
                    iced::widget::text_input::Status::Focused { .. } => ThemeColors::ACCENT,
                    _ => ThemeColors::BORDER,
                };
                iced::widget::text_input::Style {
                    background: iced::Background::Color(ThemeColors::BG_DARK),
                    border: iced::Border::default()
                        .rounded(6)
                        .color(border_color)
                        .width(1),
                    icon: ThemeColors::TEXT_MUTED,
                    placeholder: ThemeColors::TEXT_DIM,
                    value: ThemeColors::TEXT_PRIMARY,
                    selection: ThemeColors::ACCENT_DIM,
                }
            });

        // ── Method filter buttons ──
        let methods = ["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS"];
        let mut filter_buttons = row![].spacing(4);
        for method in methods {
            let is_active = self.filter_method.eq_ignore_ascii_case(method);
            let method_col = theme::method_color(method);
            let btn = button(text(method).size(10).color(
                if is_active {
                    ThemeColors::TEXT_PRIMARY
                } else {
                    method_col
                }
            ))
            .padding(Padding::from([3, 8]))
            .style(move |_theme, _status| {
                if is_active {
                    button::Style {
                        background: Some(iced::Background::Color(theme::method_color_dim(method))),
                        text_color: ThemeColors::TEXT_PRIMARY,
                        border: iced::Border::default()
                            .rounded(4)
                            .color(method_col)
                            .width(1),
                        ..button::Style::default()
                    }
                } else {
                    button::Style {
                        background: Some(iced::Background::Color(ThemeColors::BG_DARK)),
                        text_color: method_col,
                        border: iced::Border::default()
                            .rounded(4)
                            .color(ThemeColors::BORDER)
                            .width(1),
                        ..button::Style::default()
                    }
                }
            })
            .on_press(Message::FilterMethod(method.to_string()));
            filter_buttons = filter_buttons.push(btn);
        }

        let count = self.filtered_entries().len();
        let total = self.entries.len();
        let count_text = text(format!("{}/{}", count, total))
            .size(10)
            .color(ThemeColors::TEXT_MUTED);

        let filter_row = row![
            text("Filter:")
                .size(11)
                .color(ThemeColors::TEXT_SECONDARY),
            filter_buttons,
            count_text,
        ]
        .spacing(8)
        .align_y(Alignment::Center);

        // ── Empty state ──
        if self.entries.is_empty() {
            return container(
                column![
                    header,
                    search_input,
                    filter_row,
                    column![
                        lucide::history().size(40).color(ThemeColors::TEXT_DIM),
                        text("No request history yet").size(15).color(ThemeColors::TEXT_SECONDARY),
                        text("Send a request to start building history").size(12).color(ThemeColors::TEXT_MUTED),
                    ]
                    .spacing(8)
                    .align_x(Alignment::Center),
                ]
                .spacing(12)
                .padding(16),
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .into();
        }

        // ── History entries ──
        let filtered = self.filtered_entries();
        let mut list = column![].spacing(2);

        for entry in &filtered {
            let method_color = theme::method_color(&entry.method);
            let method_bg = theme::method_color_dim(&entry.method);

            let status_val = entry.status.unwrap_or(0);
            let status_color = theme::status_color(status_val);
            let status_bg = theme::status_color_dim(status_val);
            let status_text = match entry.status {
                Some(s) => format!("{}", s),
                None => "---".to_string(),
            };

            let duration_text = match entry.duration_ms {
                Some(d) => {
                    if d < 1000 {
                        format!("{}ms", d)
                    } else {
                        format!("{:.1}s", d as f64 / 1000.0)
                    }
                }
                None => "N/A".to_string(),
            };

            // URL display with smarter truncation
            let url_display: String = entry.url.chars().take(50).collect();
            let url_truncated = if entry.url.len() > 50 {
                format!("{}...", url_display)
            } else {
                url_display
            };

            let timestamp_display = entry.timestamp.chars().take(16).collect::<String>();

            // ── Method badge ──
            let method_badge = container(
                text(&entry.method)
                    .size(10)
                    .color(method_color)
                    .font(iced::Font {
                        weight: iced::font::Weight::Bold,
                        ..iced::font::Font::default()
                    }),
            )
            .padding(Padding::from([3, 6]))
            .style(move |_theme: &Theme| iced::widget::container::Style {
                background: Some(iced::Background::Color(method_bg)),
                border: iced::Border::default()
                    .rounded(4)
                    .color(method_color)
                    .width(1),
                ..iced::widget::container::Style::default()
            });

            // ── Status badge ──
            let status_badge = container(
                text(status_text.clone())
                    .size(10)
                    .color(status_color)
                    .font(iced::Font {
                        weight: iced::font::Weight::Bold,
                        ..iced::font::Font::default()
                    }),
            )
            .padding(Padding::from([3, 6]))
            .style(move |_theme: &Theme| iced::widget::container::Style {
                background: Some(iced::Background::Color(status_bg)),
                border: iced::Border::default()
                    .rounded(4)
                    .color(status_color)
                    .width(1),
                ..iced::widget::container::Style::default()
            });

            // ── Duration ──
            let duration_label = text(duration_text.clone())
                .size(11)
                .color(ThemeColors::TEXT_MUTED);

            // ── Timestamp ──
            let timestamp_label = text(timestamp_display.clone())
                .size(10)
                .color(ThemeColors::TEXT_DIM);

            // ── URL ──
            let url_label = text(url_truncated.clone())
                .size(12)
                .color(ThemeColors::TEXT_PRIMARY);

            // ── Indicators (method features) ──
            let has_body = entry
                .request_data
                .as_ref()
                .map(|d| d.contains("\"body\":"))
                .unwrap_or(false);
            let has_auth = entry
                .request_data
                .as_ref()
                .map(|d| d.contains("\"auth_type\":"))
                .unwrap_or(false);

            let mut indicators = row![].spacing(3);
            if has_body {
                indicators = indicators.push(
                    container(text("body").size(9).color(ThemeColors::CYAN))
                        .padding(Padding::from([1, 4]))
                        .style(|_theme: &Theme| iced::widget::container::Style {
                            background: Some(iced::Background::Color(Color::from_rgb(0.10, 0.20, 0.25))),
                            border: iced::Border::default().rounded(3),
                            ..iced::widget::container::Style::default()
                        }),
                );
            }
            if has_auth {
                indicators = indicators.push(
                    container(text("auth").size(9).color(ThemeColors::ORANGE))
                        .padding(Padding::from([1, 4]))
                        .style(|_theme: &Theme| iced::widget::container::Style {
                            background: Some(iced::Background::Color(Color::from_rgb(0.25, 0.18, 0.08))),
                            border: iced::Border::default().rounded(3),
                            ..iced::widget::container::Style::default()
                        }),
                );
            }

            // ── Entry content row ──
            let entry_content = row![
                method_badge,
                url_label.width(Length::Fill),
                indicators,
                status_badge,
                duration_label,
                timestamp_label,
            ]
            .spacing(8)
            .align_y(Alignment::Center);

            // ── Entry button (main clickable area) ──
            let entry_button = button(entry_content)
                .width(Length::Fill)
                .padding(Padding::from([8, 10]))
                .style(|_theme, _status| button::Style {
                    background: Some(iced::Background::Color(ThemeColors::BG_MEDIUM)),
                    border: iced::Border::default()
                        .rounded(6)
                        .color(ThemeColors::BORDER)
                        .width(1),
                    ..button::Style::default()
                })
                .on_press(Message::ResendEntry(entry.id));

            // ── View button ──
            let view_btn = button(lucide::eye().size(12))
                .padding(Padding::from([4, 8]))
                .style(|_theme, _status| button::Style {
                    background: Some(iced::Background::Color(ThemeColors::BG_LIGHT)),
                    text_color: ThemeColors::TEXT_SECONDARY,
                    border: iced::Border::default()
                        .rounded(4)
                        .color(ThemeColors::BORDER)
                        .width(1),
                    ..button::Style::default()
                })
                .on_press(Message::ViewResponse(entry.id));

            // ── Delete button (or confirmation) ──
            let delete_btn: Element<'_, Message, Theme, Renderer> =
                if self.pending_delete_entry == Some(entry.id) {
                    row![
                        button(
                            row![lucide::check().size(10), text(" Yes").size(10)].spacing(2),
                        )
                        .padding(Padding::from([3, 6]))
                        .style(|_theme, _status| button::Style {
                            background: Some(iced::Background::Color(ThemeColors::ERROR_DIM)),
                            text_color: ThemeColors::ERROR,
                            border: iced::Border::default()
                                .rounded(4)
                                .color(ThemeColors::ERROR)
                                .width(1),
                            ..button::Style::default()
                        })
                        .on_press(Message::ConfirmDeleteEntry(entry.id)),
                        button(
                            row![lucide::x().size(10), text(" No").size(10)].spacing(2),
                        )
                        .padding(Padding::from([3, 6]))
                        .style(|_theme, _status| button::Style {
                            background: Some(iced::Background::Color(ThemeColors::BG_LIGHT)),
                            text_color: ThemeColors::TEXT_SECONDARY,
                            border: iced::Border::default()
                                .rounded(4)
                                .color(ThemeColors::BORDER)
                                .width(1),
                            ..button::Style::default()
                        })
                        .on_press(Message::CancelDeleteEntry),
                    ]
                    .spacing(4)
                    .align_y(Alignment::Center)
                    .into()
                } else {
                    button(lucide::trash().size(12))
                        .padding(Padding::from([4, 6]))
                        .style(|_theme, _status| button::Style {
                            background: Some(iced::Background::Color(ThemeColors::BG_LIGHT)),
                            text_color: ThemeColors::TEXT_DIM,
                            border: iced::Border::default()
                                .rounded(4)
                                .color(ThemeColors::BORDER)
                                .width(1),
                            ..button::Style::default()
                        })
                        .on_press(Message::RequestDeleteEntry(entry.id))
                        .into()
                };

            // ── Action buttons row ──
            let actions = row![view_btn, delete_btn]
                .spacing(4)
                .align_y(Alignment::Center);

            // ── Full row with entry + actions ──
            let full_row = row![entry_button, actions]
                .spacing(6)
                .align_y(Alignment::Center)
                .width(Length::Fill);

            list = list.push(full_row);
        }

        // ── No results state ──
        if filtered.is_empty() && !self.search_query.is_empty() {
            list = list.push(
                container(
                    column![
                        lucide::search().size(32).color(ThemeColors::TEXT_DIM),
                        text(format!("No results for \"{}\"", self.search_query))
                            .size(13)
                            .color(ThemeColors::TEXT_SECONDARY),
                    ]
                    .spacing(8)
                    .align_x(Alignment::Center),
                )
                .padding(20)
                .width(Length::Fill),
            );
        }

        let mut content = column![header, search_input, filter_row].spacing(10);

        if let Some(ref entry) = self.viewing_response {
            content = content
                .push(
                    container(text(""))
                        .height(1)
                        .width(Length::Fill)
                        .style(|_theme: &Theme| iced::widget::container::Style {
                            background: Some(iced::Background::Color(ThemeColors::BORDER)),
                            ..iced::widget::container::Style::default()
                        }),
                )
                .push(Self::build_response_panel(entry));
        }

        content = content.push(scrollable(list).height(Length::Fill));

        container(content.padding(16))
            .width(Length::Fill)
            .height(Length::Fill)
            .style(|_theme: &Theme| iced::widget::container::Style {
                background: Some(iced::Background::Color(ThemeColors::BG_DARK)),
                ..iced::widget::container::Style::default()
            })
            .into()
    }
}
