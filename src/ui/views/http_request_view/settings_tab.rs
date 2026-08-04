use super::{HttpRequestView, Message};
use crate::http_client::config::RedirectPolicy;
use iced::widget::{button, column, container, row, rule, scrollable, text, text_input};
use iced::{Alignment, Color, Element, Length, Theme};
use iced_fonts::lucide;

impl HttpRequestView {
    pub(super) fn create_settings_tab_content(
        &self,
    ) -> Element<'_, Message, Theme, iced::Renderer> {
        let timeout_value = self.request_config.timeout.as_secs().to_string();
        let timeout_input = text_input("Timeout (secs)", &timeout_value)
            .on_input(Message::TimeoutChanged)
            .padding(10)
            .width(Length::Fixed(200.0));

        let follow_redirects = matches!(
            self.request_config.redirect_policy,
            RedirectPolicy::Follow | RedirectPolicy::Limited(_)
        );
        let redirect_toggle = button(if follow_redirects {
            "Follow Redirects: ON"
        } else {
            "Follow Redirects: OFF"
        })
        .on_press(Message::FollowRedirectsToggled(!follow_redirects));

        let max_redirects = match &self.request_config.redirect_policy {
            RedirectPolicy::Limited(n) => n.to_string(),
            _ => "10".to_string(),
        };
        let max_redirects_input = text_input("Max Redirects", &max_redirects)
            .on_input(Message::MaxRedirectsChanged)
            .padding(10)
            .width(Length::Fixed(200.0));

        let retry_count = self.request_config.retry.max_retries.to_string();
        let retry_count_input = text_input("Retries", &retry_count)
            .on_input(Message::RetryCountChanged)
            .padding(10)
            .width(Length::Fixed(200.0));

        let retry_backoff = self.request_config.retry.backoff_ms.to_string();
        let retry_backoff_input = text_input("Backoff (ms)", &retry_backoff)
            .on_input(Message::RetryBackoffChanged)
            .padding(10)
            .width(Length::Fixed(200.0));

        let proxy_url = self.request_config.proxy_url.as_deref().unwrap_or("");
        let proxy_input = text_input("Proxy URL (e.g. http://proxy:8080)", proxy_url)
            .on_input(Message::ProxyUrlChanged)
            .padding(10);

        let proxy_auth = self
            .request_config
            .proxy
            .as_ref()
            .and_then(|p| p.auth.as_ref());
        let proxy_username = proxy_auth.map_or("", |a| a.username.as_str());
        let proxy_password = proxy_auth.map_or("", |a| a.password.as_str());

        let proxy_username_input = text_input("Proxy Username", proxy_username)
            .on_input(Message::ProxyAuthUsernameChanged)
            .padding(10);
        let proxy_password_input = text_input("Proxy Password", proxy_password)
            .on_input(Message::ProxyAuthPasswordChanged)
            .padding(10);

        let verify_ssl = self.request_config.tls.verify_ssl;
        let cookie_store = self.request_config.cookie_store;
        let cookie_toggle = button(if cookie_store {
            "Cookie Store: ON"
        } else {
            "Cookie Store: OFF"
        })
        .on_press(Message::CookieStoreToggled(!cookie_store));

        let ssl_toggle = button(if verify_ssl {
            "Verify SSL: ON"
        } else {
            "Verify SSL: OFF (insecure)"
        })
        .on_press(Message::VerifySslToggled(!verify_ssl));

        let ssl_warning: Element<'_, Message, Theme, iced::Renderer> = if verify_ssl {
            column![].into()
        } else {
            container(
                row![
                    lucide::triangle_alert().size(14),
                    text(" SSL verification disabled. Requests may be intercepted.").size(12)
                ]
                .spacing(6)
                .align_y(Alignment::Center),
            )
            .padding(8)
            .style(move |_theme: &Theme| iced::widget::container::Style {
                background: Some(iced::Color::from_rgb(0.8, 0.2, 0.2).into()),
                text_color: Some(iced::Color::WHITE),
                ..Default::default()
            })
            .into()
        };

        let ca_cert = self
            .request_config
            .tls
            .ca_cert_path
            .as_deref()
            .unwrap_or("");
        let ca_cert_input = text_input("CA Certificate Path (optional)", ca_cert)
            .on_input(Message::CaCertPathChanged)
            .padding(10);

        let client_cert = self
            .request_config
            .tls
            .client_cert_path
            .as_deref()
            .unwrap_or("");
        let client_cert_input = text_input("Client Certificate Path (mTLS)", client_cert)
            .on_input(Message::ClientCertPathChanged)
            .padding(10);

        let client_key = self
            .request_config
            .tls
            .client_key_path
            .as_deref()
            .unwrap_or("");
        let client_key_input = text_input("Client Key Path (mTLS)", client_key)
            .on_input(Message::ClientKeyPathChanged)
            .padding(10);

        let theme_selector = iced::widget::pick_list(
            iced::highlighter::Theme::ALL,
            Some(self.highlighter_theme),
            Message::ThemeSelected,
        )
        .padding(10);

        container(scrollable(
            column![
                text("Request Settings").size(18),
                row![text("Timeout:"), timeout_input]
                    .spacing(10)
                    .align_y(Alignment::Center),
                row![redirect_toggle].spacing(10),
                row![text("Max Redirects:"), max_redirects_input]
                    .spacing(10)
                    .align_y(Alignment::Center),
                rule::horizontal(10),
                text("Retry").size(16),
                row![text("Retries:"), retry_count_input]
                    .spacing(10)
                    .align_y(Alignment::Center),
                row![text("Backoff:"), retry_backoff_input, text("ms")]
                    .spacing(10)
                    .align_y(Alignment::Center),
                rule::horizontal(10),
                text("Network").size(16),
                proxy_input,
                row![proxy_username_input, proxy_password_input]
                    .spacing(10)
                    .width(Length::Fill),
                cookie_toggle,
                ssl_toggle,
                ssl_warning,
                rule::horizontal(10),
                text("TLS / mTLS").size(16),
                ca_cert_input,
                client_cert_input,
                client_key_input,
                rule::horizontal(10),
                text("Appearance").size(16),
                row![text("Highlight Theme:"), theme_selector]
                    .spacing(10)
                    .align_y(Alignment::Center),
                rule::horizontal(10),
                text("Security").size(16),
                text("Stored secrets (OAuth2 tokens, passwords, API keys) are kept in the OS keychain.")
                    .size(12)
                    .color(Color::from_rgb(0.5, 0.5, 0.5)),
                button(
                    row![
                        lucide::trash().size(14),
                        text(" Clear All Keychain Secrets").size(13),
                    ]
                    .spacing(4),
                )
                .on_press(Message::ClearKeychainSecrets),
                rule::horizontal(10),
                text("Cookies").size(16),
                {
                    let cookie_info = if self.cookie_count > 0 {
                        format!(
                            "{} cookies across {} domains",
                            self.cookie_count, self.cookie_domain_count
                        )
                    } else {
                        "No cookies stored".to_string()
                    };
                    let info_color = if self.cookie_count > 0 {
                        Color::from_rgb(0.3, 0.7, 0.3)
                    } else {
                        Color::from_rgb(0.5, 0.5, 0.5)
                    };
                    row![
                        lucide::cookie().size(14),
                        text(cookie_info).size(13).color(info_color),
                    ]
                    .spacing(6)
                    .align_y(Alignment::Center)
                },
                button(
                    row![
                        lucide::trash().size(14),
                        text(" Clear All Cookies").size(13),
                    ]
                    .spacing(4),
                )
                .on_press(Message::ClearCookies),
                rule::horizontal(10),
                text("Sessions").size(16),
                {
                    let new_session_input =
                        text_input("New session name...", &self.new_session_name)
                            .on_input(Message::SessionNewNameChanged)
                            .padding(8)
                            .width(Length::Fill);

                    let save_btn = if self.new_session_name.trim().is_empty() {
                        button(row![lucide::save().size(12), text(" Save").size(12)].spacing(4))
                    } else {
                        button(
                            row![lucide::save().size(12), text(" Save").size(12)].spacing(4),
                        )
                        .on_press(Message::SessionSave(self.new_session_name.clone()))
                    };

                    let mut session_list = column![].spacing(4);

                    if self.sessions.is_empty() {
                        session_list = session_list.push(
                            text("No sessions saved yet")
                                .size(13)
                                .color(Color::from_rgb(0.5, 0.5, 0.5)),
                        );
                    } else {
                        for session in &self.sessions {
                            let is_renaming = self
                                .renaming_session
                                .as_ref()
                                .is_some_and(|s| s == &session.id);

                            let is_pending_delete = self
                                .pending_delete_session
                                .as_ref()
                                .is_some_and(|s| s == &session.id);

                            let session_row = if is_renaming {
                                let rename_input =
                                    text_input("Rename...", &self.rename_value)
                                        .on_input(Message::SessionRenameValueChanged)
                                        .padding(4)
                                        .width(Length::Fill);
                                row![
                                    rename_input,
                                    button(text("\u{2713}").size(12))
                                        .on_press(Message::SessionRenameConfirm),
                                    button(text("\u{2717}").size(12))
                                        .on_press(Message::SessionRenameCancel),
                                ]
                                .spacing(4)
                                .align_y(Alignment::Center)
                            } else if is_pending_delete {
                                row![
                                    text(format!("Delete \"{}\"?", session.name)).size(13),
                                    button(text("Yes").size(12).color(Color::from_rgb(0.9, 0.3, 0.3)))
                                        .on_press(Message::SessionConfirmDelete(
                                            session.id.clone(),
                                        )),
                                    button(text("No").size(12))
                                        .on_press(Message::SessionCancelDelete),
                                ]
                                .spacing(4)
                                .align_y(Alignment::Center)
                            } else {
                                row![
                                    button(text(&session.name).size(13))
                                        .on_press(Message::SessionLoad(session.id.clone())),
                                    button(lucide::pencil().size(11))
                                        .on_press(Message::SessionRenameStart(session.id.clone())),
                                    button(lucide::trash().size(11))
                                        .on_press(Message::SessionDelete(session.id.clone())),
                                ]
                                .spacing(4)
                                .align_y(Alignment::Center)
                            };

                            session_list = session_list.push(session_row);
                        }
                    }

                    column![
                        row![new_session_input, save_btn]
                            .spacing(8)
                            .align_y(Alignment::Center),
                        session_list,
                    ]
                    .spacing(8)
                },
                rule::horizontal(10),
                button(row![lucide::rotate_ccw().size(14), text(" Reset to Defaults")].spacing(4))
                    .on_press(Message::ResetSettings),
            ]
            .spacing(15)
            .padding(20),
        ))
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }
}
