use super::{HttpRequestView, Message, TabId};
use crate::data::auth::Auth;
use crate::ui::request_status::RequestStatus;
use crate::ui::theme::status_color;
use iced::widget::{button, column, container, pick_list, row, rule, scrollable, text};
use iced::{Alignment, Color, Element, Length, Theme};
use iced_aw::{TabLabel, Tabs};
use iced_fonts::lucide;

impl HttpRequestView {
    pub(crate) fn view(&self) -> Element<'_, Message, Theme, iced::Renderer> {
        let body_tab_content = self.create_body_tab_content();
        let auth_tab_content = self.create_auth_tab_content();
        let settings_tab_content = self.create_settings_tab_content();

        let tabs = Tabs::new(Message::TabSelected)
            .push(
                TabId::Body,
                TabLabel::Text("Body".to_string()),
                body_tab_content,
            )
            .push(
                TabId::Headers,
                TabLabel::Text("Headers".to_string()),
                container(self.headers_editor.view().map(Message::HeadersEditor))
                    .padding(10)
                    .width(Length::Fill)
                    .height(Length::Fill),
            )
            .push(
                TabId::Params,
                TabLabel::Text("Params".to_string()),
                container(self.params_editor.view().map(Message::ParamsEditor))
                    .padding(10)
                    .width(Length::Fill)
                    .height(Length::Fill),
            )
            .push(
                TabId::Authorization,
                TabLabel::Text("Authorization".to_string()),
                container(auth_tab_content)
                    .padding(10)
                    .width(Length::Fill)
                    .height(Length::Fill),
            )
            .push(
                TabId::Scripts,
                TabLabel::Text("Scripts".to_string()),
                container(self.create_scripts_tab_content())
                    .padding(10)
                    .width(Length::Fill)
                    .height(Length::Fill),
            )
            .push(
                TabId::Cookies,
                TabLabel::Text(format!("Cookies ({})", self.cookie_count)),
                container(self.create_cookies_tab_content())
                    .padding(10)
                    .width(Length::Fill)
                    .height(Length::Fill),
            )
            .push(
                TabId::Settings,
                TabLabel::Text("Settings".to_string()),
                container(settings_tab_content)
                    .padding(10)
                    .width(Length::Fill)
                    .height(Length::Fill),
            )
            .set_active_tab(&self.active_tab)
            .width(Length::Fill);

        let response_area = self.create_response_area();

        let method_colored = text(self.method.as_str())
            .size(16)
            .color(super::method_color(self.method.as_str()));

        let status_text = if let Some(status) = self.status_code {
            let color = status_color(status);
            text(format!("  {}  ", status)).size(14).color(color)
        } else {
            text("".to_string()).size(14)
        };

        let content_type_badge = self.build_content_type_badge();
        let duration_text = self.build_duration_text();
        let size_text = self.build_size_text();
        let http_warning = self.build_http_warning();
        let copy_button = self.build_copy_button();
        let download_button = self.build_download_button();
        let image_preview_button = self.build_image_preview_button();
        let wrap_toggle = self.build_wrap_toggle();

        let main_column = column![
            iced::widget::image::Image::new(self.logo_handle.clone())
                .width(Length::Fixed(100.0))
                .height(Length::Fixed(100.0)),
            {
                let send_btn = button(row![lucide::send().size(14), text(" Send")].spacing(4))
                    .on_press(Message::SendRequest);
                let cancel_btn = button(row![lucide::x().size(14), text(" Cancel")].spacing(4))
                    .on_press(Message::CancelRequest);
                let code_btn = button(row![lucide::code().size(14), text(" Code")].spacing(4))
                    .on_press(Message::ShowSnippets);

                if matches!(self.request_status, RequestStatus::Loading { .. }) {
                    row![
                        pick_list(
                            &super::HTTP_METHODS[..],
                            Some(self.method.as_str()),
                            |s: &str| { Message::MethodSelected(s.to_string()) }
                        )
                        .padding(10),
                        iced::widget::text_input("URL or paste cURL command", &self.url_input)
                            .on_input(Message::UrlInputChanged)
                            .on_submit(Message::SendRequest)
                            .padding(10),
                        cancel_btn,
                        code_btn,
                    ]
                } else {
                    row![
                        pick_list(
                            &super::HTTP_METHODS[..],
                            Some(self.method.as_str()),
                            |s: &str| { Message::MethodSelected(s.to_string()) }
                        )
                        .padding(10),
                        iced::widget::text_input("URL or paste cURL command", &self.url_input)
                            .on_input(Message::UrlInputChanged)
                            .on_submit(Message::SendRequest)
                            .padding(10),
                        send_btn,
                        code_btn,
                    ]
                }
            }
            .spacing(10)
            .padding(iced::Padding::from([16, 10])),
            http_warning,
            tabs.height(Length::Fixed(300.0)),
            rule::horizontal(10),
            column![
                row![
                    method_colored,
                    status_text,
                    content_type_badge,
                    duration_text,
                    text(" | ").size(14),
                    size_text,
                    row![
                        copy_button,
                        download_button,
                        image_preview_button,
                        wrap_toggle
                    ]
                    .align_y(Alignment::Center),
                ]
                .spacing(8)
                .padding(10)
                .align_y(Alignment::Center),
                response_area,
            ]
            .height(Length::Fill),
        ]
        .align_x(Alignment::Center);

        if self.show_snippets {
            let snippets_panel = self.create_snippets_panel();
            row![
                scrollable(main_column.width(Length::FillPortion(3))),
                rule::vertical(1),
                container(snippets_panel)
                    .width(Length::FillPortion(2))
                    .height(Length::Fill),
            ]
            .into()
        } else {
            scrollable(main_column).into()
        }
    }

    fn build_content_type_badge(&self) -> Element<'_, Message, Theme, iced::Renderer> {
        if let Some(ct) = &self.content_type {
            let short_ct = if ct.contains("json") {
                "JSON"
            } else if ct.contains("html") {
                "HTML"
            } else if ct.contains("xml") {
                "XML"
            } else if ct.contains("image/") {
                "IMG"
            } else if ct.contains("pdf") {
                "PDF"
            } else if ct.contains("octet-stream") {
                "BIN"
            } else if ct.contains("text/") {
                "TEXT"
            } else {
                "?"
            };
            Element::from(
                container(
                    text(short_ct)
                        .size(10)
                        .color(Color::from_rgb(1.0, 1.0, 1.0)),
                )
                .padding(iced::Padding::from([2, 6]))
                .style(move |_: &Theme| iced::widget::container::Style {
                    background: Some(iced::Color::from_rgb(0.3, 0.3, 0.5).into()),
                    border: iced::Border::default().rounded(4),
                    ..iced::widget::container::Style::default()
                }),
            )
        } else {
            Element::from(column![])
        }
    }

    fn build_duration_text(&self) -> Element<'_, Message, Theme, iced::Renderer> {
        text(format!(
            "{}ms",
            self.response_duration
                .map(|d| d.as_millis().to_string())
                .unwrap_or_else(|| "N/A".to_string())
        ))
        .size(14)
        .into()
    }

    fn build_size_text(&self) -> Element<'_, Message, Theme, iced::Renderer> {
        text(
            self.response_size
                .map(|s| {
                    if s > 1024 {
                        format!("{:.1} KB", s as f64 / 1024.0)
                    } else {
                        format!("{} B", s)
                    }
                })
                .unwrap_or_else(|| "N/A".to_string()),
        )
        .size(14)
        .into()
    }

    fn build_http_warning(&self) -> Element<'_, Message, Theme, iced::Renderer> {
        let has_auth = !matches!(self.auth, Auth::None);
        let is_http = self.url_input.starts_with("http://");
        if is_http && has_auth {
            container(
                row![
                    lucide::triangle_alert()
                        .size(14)
                        .color(Color::from_rgb(1.0, 0.8, 0.0)),
                    text("Warning: Sending credentials over plaintext HTTP")
                        .size(12)
                        .color(Color::from_rgb(1.0, 0.8, 0.0)),
                ]
                .spacing(8),
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
    }

    fn build_copy_button(&self) -> Element<'_, Message, Theme, iced::Renderer> {
        if matches!(
            self.request_status,
            RequestStatus::Success | RequestStatus::Error(_)
        ) {
            button(row![lucide::copy().size(14), text(" Copy")].spacing(4))
                .on_press(Message::CopyResponse)
                .into()
        } else {
            column![].into()
        }
    }

    fn build_download_button(&self) -> Element<'_, Message, Theme, iced::Renderer> {
        if matches!(self.request_status, RequestStatus::Success) {
            let is_binary = self
                .content_type
                .as_deref()
                .map(|ct| {
                    ct.contains("image/")
                        || ct.contains("application/octet-stream")
                        || ct.contains("application/pdf")
                        || ct.contains("application/zip")
                        || ct.contains("application/gzip")
                        || ct.contains("audio/")
                        || ct.contains("video/")
                })
                .unwrap_or(false);
            if is_binary {
                button(row![lucide::download().size(14), text(" Save File")].spacing(4))
                    .on_press(Message::DownloadResponse)
                    .into()
            } else {
                column![].into()
            }
        } else {
            column![].into()
        }
    }

    fn build_image_preview_button(&self) -> Element<'_, Message, Theme, iced::Renderer> {
        if self.image_preview_handle.is_some() {
            button(
                row![
                    if self.show_image_preview {
                        lucide::eye_off().size(14)
                    } else {
                        lucide::eye().size(14)
                    },
                    text(if self.show_image_preview {
                        "Hide Image"
                    } else {
                        "Show Image"
                    })
                    .size(11),
                ]
                .spacing(4),
            )
            .on_press(Message::ToggleImagePreview)
            .into()
        } else {
            column![].into()
        }
    }

    fn build_wrap_toggle(&self) -> Element<'_, Message, Theme, iced::Renderer> {
        if matches!(self.request_status, RequestStatus::Success) {
            button(
                row![
                    lucide::wrap_text().size(14),
                    text(if self.word_wrap {
                        "Wrap ON"
                    } else {
                        "Wrap OFF"
                    })
                    .size(11),
                ]
                .spacing(4),
            )
            .on_press(Message::ToggleWordWrap)
            .into()
        } else {
            column![].into()
        }
    }
}
