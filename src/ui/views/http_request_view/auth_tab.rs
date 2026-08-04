use super::{HttpRequestView, Message};
use crate::data::auth::Auth;
use crate::data::auth_input::AuthInput;
use crate::ui::components::auth_panel;
use iced::widget::{button, column, container, pick_list, row, rule, scrollable, text, text_input};
use iced::{Alignment, Color, Element, Length, Theme};
use iced_fonts::lucide;

impl HttpRequestView {
    pub(super) fn create_auth_tab_content(&self) -> Element<'_, Message, Theme, iced::Renderer> {
        let oauth2_content: Element<'_, Message, Theme, iced::Renderer> = match &self.auth {
            Auth::OAuth2(config) => {
                let grant_type_fields = match config.grant_type {
                    crate::data::auth::OAuth2GrantType::AuthorizationCode => column![
                        text_input("Authorization URL", &config.auth_url)
                            .on_input(|u| Message::AuthInputChanged(AuthInput::OAuth2AuthUrl(u)))
                            .padding(10),
                        text_input("Token URL", &config.token_url)
                            .on_input(|u| Message::AuthInputChanged(AuthInput::OAuth2TokenUrl(u)))
                            .padding(10),
                        text_input("Redirect URI", &config.redirect_uri)
                            .on_input(|u| Message::AuthInputChanged(AuthInput::OAuth2RedirectUri(
                                u
                            )))
                            .padding(10),
                        row![
                            text("PKCE:"),
                            button(if config.pkce_enabled { "ON" } else { "OFF" }).on_press(
                                Message::AuthInputChanged(AuthInput::OAuth2PkceEnabled(
                                    !config.pkce_enabled
                                ))
                            ),
                        ]
                        .spacing(10)
                        .align_y(Alignment::Center),
                        button(row![lucide::key().size(14), text(" Get Authorization")].spacing(4))
                            .on_press(Message::OAuth2StartAuth),
                    ]
                    .spacing(10),
                    crate::data::auth::OAuth2GrantType::ClientCredentials => column![
                        text_input("Token URL", &config.token_url)
                            .on_input(|u| Message::AuthInputChanged(AuthInput::OAuth2TokenUrl(u)))
                            .padding(10),
                        text_input("Scopes (space-separated)", &config.scopes)
                            .on_input(|s| Message::AuthInputChanged(AuthInput::OAuth2Scopes(s)))
                            .padding(10),
                        button(row![lucide::key().size(14), text(" Get Token")].spacing(4))
                            .on_press(Message::OAuth2RefreshToken),
                    ]
                    .spacing(10),
                    crate::data::auth::OAuth2GrantType::DeviceCode => column![
                        text_input("Device Auth URL", &config.device_auth_url)
                            .on_input(|u| Message::AuthInputChanged(
                                AuthInput::OAuth2DeviceAuthUrl(u)
                            ))
                            .padding(10),
                        if config.user_code.is_empty() {
                            Element::from(
                                button(
                                    row![
                                        lucide::smartphone().size(14),
                                        text(" Start Device Authorization")
                                    ]
                                    .spacing(4),
                                )
                                .on_press(Message::OAuth2StartDeviceAuth),
                            )
                        } else {
                            let poll_button_text = if config.auto_polling {
                                "Stop Auto-Polling"
                            } else {
                                "Start Auto-Polling"
                            };
                            let poll_button_icon = if config.auto_polling {
                                lucide::square().size(12)
                            } else {
                                lucide::refresh_cw().size(12)
                            };
                            Element::from(
                                column![
                                    container(
                                        text(format!("  {}  ", config.user_code))
                                            .size(24)
                                            .color(Color::from_rgb(0.0, 0.5, 1.0))
                                    )
                                    .padding(15)
                                    .center_x(Length::Fill)
                                    .style(iced::widget::container::rounded_box),
                                    text(format!("Open: {}", config.verification_uri)).size(12),
                                    button(
                                        row![lucide::copy().size(12), text(" Copy User Code")]
                                            .spacing(4)
                                    )
                                    .on_press({
                                        let code = config.user_code.clone();
                                        Message::OAuth2CopyUserCode(code)
                                    }),
                                    button(
                                        row![poll_button_icon, text(poll_button_text)].spacing(4)
                                    )
                                    .on_press(Message::OAuth2AutoPollToggle(!config.auto_polling)),
                                    if config.auto_polling {
                                        Element::from(
                                            text("Polling...")
                                                .size(11)
                                                .color(Color::from_rgb(0.2, 0.7, 0.3)),
                                        )
                                    } else {
                                        Element::from(column![])
                                    },
                                ]
                                .spacing(8),
                            )
                        },
                    ]
                    .spacing(10),
                    crate::data::auth::OAuth2GrantType::Implicit => column![
                        text_input("Authorization URL", &config.auth_url)
                            .on_input(|u| Message::AuthInputChanged(AuthInput::OAuth2AuthUrl(u)))
                            .padding(10),
                        text_input("Redirect URI", &config.redirect_uri)
                            .on_input(|u| Message::AuthInputChanged(AuthInput::OAuth2RedirectUri(
                                u
                            )))
                            .padding(10),
                        text_input("Scopes (space-separated)", &config.scopes)
                            .on_input(|s| Message::AuthInputChanged(AuthInput::OAuth2Scopes(s)))
                            .padding(10),
                        button(row![lucide::key().size(14), text(" Get Authorization")].spacing(4))
                            .on_press(Message::OAuth2StartAuth),
                    ]
                    .spacing(10),
                };

                column![
                    row![
                        text("OAuth 2.0").size(16),
                        pick_list(
                            &crate::data::auth::OAuth2GrantType::ALL[..],
                            Some(config.grant_type.clone()),
                            |gt| Message::AuthInputChanged(AuthInput::OAuth2GrantType(gt)),
                        )
                        .padding(10),
                    ]
                    .spacing(10)
                    .align_y(Alignment::Center),
                    text_input("Client ID", &config.client_id)
                        .on_input(|id| Message::AuthInputChanged(AuthInput::OAuth2ClientId(id)))
                        .padding(10),
                    text_input("Client Secret", &config.client_secret)
                        .on_input(|s| Message::AuthInputChanged(AuthInput::OAuth2ClientSecret(s)))
                        .padding(10)
                        .secure(true),
                    grant_type_fields,
                    rule::horizontal(10),
                    text("Tokens").size(14),
                    row![
                        text_input("Access Token", &config.access_token)
                            .on_input(|t| Message::AuthInputChanged(AuthInput::OAuth2AccessToken(
                                t
                            )))
                            .padding(10)
                            .secure(true),
                        button(lucide::copy().size(14)).on_press({
                            let token = config.access_token.clone();
                            Message::OAuth2CopyAccessToken(token)
                        }),
                    ]
                    .spacing(4)
                    .align_y(Alignment::Center),
                    row![
                        text_input("Refresh Token", &config.refresh_token)
                            .on_input(|t| Message::AuthInputChanged(AuthInput::OAuth2RefreshToken(
                                t
                            )))
                            .padding(10)
                            .secure(true),
                        button(lucide::copy().size(14)).on_press({
                            let token = config.refresh_token.clone();
                            Message::OAuth2CopyRefreshToken(token)
                        }),
                    ]
                    .spacing(4)
                    .align_y(Alignment::Center),
                    if config.status.to_string().is_empty() {
                        Element::from(column![])
                    } else {
                        Element::from(text(config.status.to_string()).size(12).color(
                            match &config.status {
                                crate::data::auth::OAuth2Status::Error(_) => {
                                    Color::from_rgb(0.8, 0.2, 0.2)
                                }
                                crate::data::auth::OAuth2Status::Success(_) => {
                                    Color::from_rgb(0.2, 0.7, 0.3)
                                }
                                crate::data::auth::OAuth2Status::Loading => {
                                    Color::from_rgb(0.8, 0.7, 0.1)
                                }
                                _ => Color::from_rgb(0.5, 0.5, 0.5),
                            },
                        ))
                    },
                ]
                .spacing(10)
                .into()
            }
            _ => column![].into(),
        };

        let panel = auth_panel::auth_panel(
            &self.auth,
            self.show_bearer_token,
            self.show_api_key_value,
            Message::AuthTypeSelected,
            |t| Message::AuthInputChanged(AuthInput::BearerToken(t)),
            |_| Message::ToggleBearerTokenVisible,
            |u| Message::AuthInputChanged(AuthInput::BasicUser(u)),
            |p| Message::AuthInputChanged(AuthInput::BasicPass(p)),
            |k| Message::AuthInputChanged(AuthInput::ApiKeyKey(k)),
            |v| Message::AuthInputChanged(AuthInput::ApiKeyValue(v)),
            |loc| Message::AuthInputChanged(AuthInput::ApiKeyLocation(loc)),
            |_| Message::ToggleApiKeyValueVisible,
            |u| Message::AuthInputChanged(AuthInput::DigestUser(u)),
            |p| Message::AuthInputChanged(AuthInput::DigestPass(p)),
            oauth2_content,
        );

        container(scrollable(panel))
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}
