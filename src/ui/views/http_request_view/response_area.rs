use super::{HttpRequestView, Message, ResponseTab};
use crate::ui::request_status::RequestStatus;
use crate::ui::theme::status_color;
use iced::widget::text_editor;
use iced::widget::{button, column, container, row, rule, scrollable, text, text_input};
use iced::{Alignment, Color, Element, Length, Theme};
use iced_aw::{ContextMenu, TabLabel, Tabs};
use iced_fonts::lucide;

impl HttpRequestView {
    pub(super) fn create_response_area(&self) -> Element<'_, Message, Theme, iced::Renderer> {
        match &self.request_status {
            RequestStatus::Idle => {
                let idle_content = column![
                    text("Ready to send")
                        .size(18)
                        .color(Color::from_rgb(0.6, 0.6, 0.6)),
                    text(
                        "Enter a URL above and click Send, or paste a cURL command in the URL bar"
                    )
                    .size(13)
                    .color(Color::from_rgb(0.45, 0.45, 0.45)),
                ]
                .spacing(12)
                .align_x(Alignment::Center);

                container(idle_content)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .align_x(Alignment::Center)
                    .align_y(Alignment::Center)
                    .into()
            }
            RequestStatus::Loading { started_at } => {
                let elapsed = started_at.elapsed().as_millis();
                let elapsed_text = if elapsed < 1000 {
                    format!("{elapsed}ms")
                } else {
                    format!("{:.1}s", elapsed as f64 / 1000.0)
                };
                container(
                    column![
                        iced_aw::Spinner::new().width(32).height(32),
                        text(format!("Sending request... ({elapsed_text})")).size(14),
                    ]
                    .spacing(8)
                    .align_x(Alignment::Center),
                )
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(Alignment::Center)
                .align_y(Alignment::Center)
                .into()
            }
            RequestStatus::Success => {
                let search_bar = if self.show_response_search {
                    let match_info = if self.response_search_matches.is_empty() {
                        text("No matches")
                            .size(12)
                            .color(Color::from_rgb(0.5, 0.5, 0.5))
                    } else {
                        text(format!(
                            "{}/{}",
                            self.response_search_index + 1,
                            self.response_search_matches.len()
                        ))
                        .size(12)
                        .color(Color::from_rgb(0.3, 0.7, 0.3))
                    };
                    Some(
                        row![
                            button(lucide::x().size(12)).on_press(Message::ToggleResponseSearch),
                            text_input("Search...", &self.response_search_query)
                                .on_input(Message::ResponseSearchChanged)
                                .padding(5)
                                .width(Length::Fill),
                            match_info,
                            button(lucide::chevron_up().size(12)).on_press(Message::SearchPrev),
                            button(lucide::chevron_down().size(12)).on_press(Message::SearchNext),
                        ]
                        .spacing(6)
                        .align_y(Alignment::Center)
                        .padding(iced::Padding::from([4, 8])),
                    )
                } else {
                    None
                };

                let response_tabs = Tabs::new(Message::ResponseTabSelected)
                    .push(ResponseTab::Body, TabLabel::Text("Body".to_string()), {
                        if self.show_image_preview {
                            if let Some(handle) = &self.image_preview_handle {
                                let img = iced::widget::image::Image::new(handle.clone())
                                    .width(Length::Fill)
                                    .height(Length::Fill)
                                    .content_fit(iced::ContentFit::Contain);
                                container(img)
                                    .width(Length::Fill)
                                    .height(Length::Fill)
                                    .center(Length::Fill)
                            } else {
                                container(text("No image available"))
                                    .width(Length::Fill)
                                    .height(Length::Fill)
                                    .align_x(Alignment::Center)
                                    .align_y(Alignment::Center)
                            }
                        } else if self.word_wrap {
                            let body_text = self.response_body_editor.text();
                            let wrapped_text =
                                text(body_text).size(13).font(iced::Font::MONOSPACE);
                            let context_menu =
                                ContextMenu::new(scrollable(wrapped_text), || {
                                    column![button(
                                        row![lucide::copy().size(12), text(" Copy Body")]
                                            .spacing(4)
                                    )
                                    .on_press(Message::CopyBody)]
                                    .into()
                                });
                            container(context_menu)
                                .width(Length::Fill)
                                .height(Length::Fill)
                        } else {
                            let body_len = self.response_body_editor.text().len();
                            let syntax = self
                                .content_type
                                .as_deref()
                                .map_or("text", super::helpers::response_content_type_to_syntax);
                            let is_truncated = self.highlight_content.is_some();
                            let highlight_ref = self
                                .highlight_content
                                .as_ref()
                                .unwrap_or(&self.response_body_editor);
                            let editor = text_editor(highlight_ref)
                                .on_action(Message::ResponseContentChanged)
                                .highlight(syntax, self.highlighter_theme);
                            let truncated_banner = if is_truncated {
                                Some(
                                    container(
                                        row![
                                            lucide::info()
                                                .size(12)
                                                .color(Color::from_rgb(0.3, 0.6, 0.9)),
                                            text(format!(
                                                "Showing first {:.0} KB of {:.0} KB with syntax highlighting. Full response available via Copy.",
                                                500_000.0 / 1024.0,
                                                body_len as f64 / 1024.0
                                            ))
                                            .size(11)
                                            .color(Color::from_rgb(0.5, 0.6, 0.8)),
                                        ]
                                        .spacing(6)
                                        .align_y(Alignment::Center),
                                    )
                                    .padding(iced::Padding::from([6, 10]))
                                    .style(|_: &Theme| iced::widget::container::Style {
                                        background: Some(
                                            Color::from_rgb(0.12, 0.15, 0.22).into(),
                                        ),
                                        border: iced::Border::default()
                                            .rounded(4)
                                            .color(Color::from_rgb(0.2, 0.3, 0.5))
                                            .width(1),
                                        ..iced::widget::container::Style::default()
                                    }),
                                )
                            } else {
                                None
                            };
                            let context_menu =
                                ContextMenu::new(scrollable(editor), || {
                                    column![
                                        button(
                                            row![
                                                lucide::copy().size(12),
                                                text(" Copy Selection")
                                            ]
                                            .spacing(4)
                                        )
                                        .on_press(Message::CopySelection),
                                        button(
                                            row![lucide::copy().size(12), text(" Copy Body")]
                                                .spacing(4)
                                        )
                                        .on_press(Message::CopyBody),
                                    ]
                                    .into()
                                });
                            if let Some(banner) = truncated_banner {
                                container(column![banner, context_menu])
                                    .width(Length::Fill)
                                    .height(Length::Fill)
                            } else {
                                container(context_menu)
                                    .width(Length::Fill)
                                    .height(Length::Fill)
                            }
                        }
                    })
                    .push(
                        ResponseTab::Headers,
                        TabLabel::Text("Headers".to_string()),
                        self.create_response_headers_view(),
                    )
                    .push(
                        ResponseTab::Timeline,
                        TabLabel::Text("Timeline".to_string()),
                        self.create_response_timeline_view(),
                    )
                    .set_active_tab(&self.active_response_tab)
                    .width(Length::Fill)
                    .height(Length::Fill);

                let response_content = if let Some(search_bar) = search_bar {
                    column![search_bar, response_tabs]
                        .spacing(0)
                        .width(Length::Fill)
                        .height(Length::Fill)
                } else {
                    column![response_tabs]
                        .width(Length::Fill)
                        .height(Length::Fill)
                };

                container(response_content)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .into()
            }
            RequestStatus::Error(error_message) => {
                let error_content = column![
                    text("Request Failed")
                        .size(16)
                        .color(Color::from_rgb(0.8, 0.2, 0.2)),
                    text(error_message.clone()).size(13),
                    row![
                        button(row![lucide::copy().size(12), text(" Copy Error")].spacing(4))
                            .on_press(Message::CopyError(error_message.clone())),
                        button(row![lucide::refresh_cw().size(12), text(" Retry")].spacing(4))
                            .on_press(Message::SendRequest),
                    ]
                    .spacing(8),
                ]
                .spacing(12)
                .align_x(Alignment::Center);

                container(error_content)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .align_x(Alignment::Center)
                    .align_y(Alignment::Center)
                    .into()
            }
        }
    }

    pub(super) fn create_response_headers_view(
        &self,
    ) -> Element<'_, Message, Theme, iced::Renderer> {
        if let Some(response) = &self.last_response {
            if response.headers.is_empty() {
                return container(text("No headers available."))
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .align_x(Alignment::Center)
                    .align_y(Alignment::Center)
                    .into();
            }

            let cookie_count = response
                .headers
                .iter()
                .filter(|(k, _)| k.eq_ignore_ascii_case("set-cookie"))
                .count();

            let header_count_text = if cookie_count > 0 {
                format!(
                    "{} headers ({} cookies)",
                    response.headers.len(),
                    cookie_count
                )
            } else {
                format!("{} headers", response.headers.len())
            };

            let header_count = text(header_count_text)
                .size(11)
                .color(Color::from_rgb(0.5, 0.5, 0.5));

            let header_row = row![
                container(
                    text("Name")
                        .size(11)
                        .color(Color::from_rgb(0.55, 0.65, 0.85))
                )
                .width(Length::FillPortion(2)),
                container(
                    text("Value")
                        .size(11)
                        .color(Color::from_rgb(0.55, 0.65, 0.85))
                )
                .width(Length::FillPortion(3)),
            ]
            .spacing(1)
            .padding(iced::Padding::from([6, 8]));

            let mut rows = column![header_row].spacing(0);
            for (i, (key, value)) in response.headers.iter().enumerate() {
                let is_set_cookie = key.eq_ignore_ascii_case("set-cookie");
                let bg = if is_set_cookie {
                    Color::from_rgba(0.15, 0.6, 0.15, 0.18)
                } else if i % 2 == 0 {
                    Color::from_rgba(0.55, 0.65, 1.0, 0.06)
                } else {
                    Color::from_rgba(0.0, 0.0, 0.0, 0.0)
                };
                let key_color = if is_set_cookie {
                    Color::from_rgb(0.35, 0.88, 0.35)
                } else {
                    Color::from_rgb(0.45, 0.7, 1.0)
                };
                let row = row![
                    container(
                        row![
                            if is_set_cookie {
                                Element::from(
                                    lucide::cookie()
                                        .size(12)
                                        .color(Color::from_rgb(0.35, 0.88, 0.35)),
                                )
                            } else {
                                Element::from(column![])
                            },
                            text(key).size(13).color(key_color),
                        ]
                        .spacing(4)
                        .align_y(Alignment::Center)
                    )
                    .width(Length::FillPortion(2))
                    .padding(iced::Padding::from([5, 8]))
                    .style(move |_: &Theme| iced::widget::container::Style {
                        background: Some(bg.into()),
                        ..iced::widget::container::Style::default()
                    }),
                    container(text(value).size(13))
                        .width(Length::FillPortion(3))
                        .padding(iced::Padding::from([5, 8]))
                        .style(move |_: &Theme| iced::widget::container::Style {
                            background: Some(bg.into()),
                            ..iced::widget::container::Style::default()
                        }),
                ]
                .spacing(1);
                rows = rows.push(row);
            }

            column![
                header_count,
                container(scrollable(rows).width(Length::Fill).height(Length::Fill))
                    .padding(1)
                    .style(|_: &Theme| iced::widget::container::Style {
                        border: iced::Border::default()
                            .rounded(4)
                            .color(Color::from_rgb(0.3, 0.3, 0.35)),
                        ..iced::widget::container::Style::default()
                    }),
            ]
            .spacing(6)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
        } else {
            container(text("No headers available."))
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(Alignment::Center)
                .align_y(Alignment::Center)
                .into()
        }
    }

    pub(super) fn create_response_timeline_view(
        &self,
    ) -> Element<'_, Message, Theme, iced::Renderer> {
        if let Some(response) = &self.last_response {
            let mut items = column![].spacing(8);

            items = items.push(
                row![
                    text("Status:")
                        .size(14)
                        .color(Color::from_rgb(0.5, 0.5, 0.5)),
                    text(response.status.to_string())
                        .size(14)
                        .color(status_color(response.status)),
                ]
                .spacing(8),
            );

            items = items.push(
                row![
                    text("Duration:")
                        .size(14)
                        .color(Color::from_rgb(0.5, 0.5, 0.5)),
                    text(if response.duration.as_millis() > 0 {
                        format!("{:.2?}", response.duration)
                    } else if let Some(started) = match &self.request_status {
                        crate::ui::request_status::RequestStatus::Loading { started_at } =>
                            Some(*started_at),
                        _ => None,
                    } {
                        let elapsed = started.elapsed();
                        format!("{elapsed:.2?}")
                    } else {
                        "N/A".to_string()
                    })
                    .size(14),
                ]
                .spacing(8),
            );

            let size_str = if response.size > 1024 {
                format!("{:.2} KB", response.size as f64 / 1024.0)
            } else {
                format!("{} bytes", response.size)
            };
            items = items.push(
                row![
                    text("Size:").size(14).color(Color::from_rgb(0.5, 0.5, 0.5)),
                    text(size_str).size(14),
                ]
                .spacing(8),
            );

            items = items.push(
                row![
                    text("URL:").size(14).color(Color::from_rgb(0.5, 0.5, 0.5)),
                    text(&response.url).size(14),
                ]
                .spacing(8),
            );

            items = items.push(
                row![
                    text("Method:")
                        .size(14)
                        .color(Color::from_rgb(0.5, 0.5, 0.5)),
                    text(response.method.to_string())
                        .size(14)
                        .color(super::method_color(&response.method.to_string())),
                ]
                .spacing(8),
            );

            if !response.redirect_chain.is_empty() {
                items = items.push(rule::horizontal(5));
                items = items.push(
                    text(format!(
                        "Redirect Chain ({} hops):",
                        response.redirect_chain.len()
                    ))
                    .size(14)
                    .color(Color::from_rgb(0.5, 0.5, 0.5)),
                );
                for (i, url) in response.redirect_chain.iter().enumerate() {
                    items = items.push(text(format!("  {}. {}", i + 1, url)).size(13));
                }
            }

            container(scrollable(items))
                .padding(10)
                .width(Length::Fill)
                .height(Length::Fill)
                .into()
        } else {
            container(text("No timeline available."))
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(Alignment::Center)
                .align_y(Alignment::Center)
                .into()
        }
    }
}
