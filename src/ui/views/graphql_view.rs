use crate::data::auth::Auth;
use crate::data::auth::AuthType;
use crate::http_client::config::RequestConfig;
use crate::protocols::graphql::{GraphQLRequest, GraphQLResponse};
use crate::ui::components::key_value_editor::{self, KeyValueEditor};
use crate::ui::theme::method_color;
use base64::{engine::general_purpose, Engine as _};
use iced::highlighter;
use iced::widget::text_editor;
use iced::{
    widget::{button, column, container, pick_list, row, rule, scrollable, text, text_input},
    Alignment, Color, Element, Length, Renderer, Theme,
};
use iced_aw::{ContextMenu, TabLabel, Tabs};
use iced_fonts::lucide;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TabId {
    #[default]
    Query,
    Variables,
    Headers,
    Authorization,
}

impl std::fmt::Display for TabId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TabId::Query => write!(f, "Query"),
            TabId::Variables => write!(f, "Variables"),
            TabId::Headers => write!(f, "Headers"),
            TabId::Authorization => write!(f, "Authorization"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ResponseTab {
    #[default]
    Body,
    Headers,
}

#[derive(Debug, Clone)]
pub enum Message {
    UrlInputChanged(String),
    QueryChanged(text_editor::Action),
    VariablesChanged(text_editor::Action),
    OperationNameChanged(String),
    TabSelected(TabId),
    ResponseTabSelected(ResponseTab),
    HeadersEditor(key_value_editor::Message),
    AuthTypeSelected(AuthType),
    AuthInputChanged(AuthInput),
    SendRequest,
    SetLoading,
    ResponseReceived(
        #[allow(clippy::type_complexity)]
        Result<
            (
                GraphQLResponse,
                u16,
                Vec<(String, String)>,
                std::time::Duration,
                u64,
            ),
            String,
        >,
    ),
    CopyResponse,
    #[allow(dead_code)]
    CopyHeaders,
    CopyBody,
    CopySelection,
    ResponseContentChanged(text_editor::Action),
    ToggleWordWrap,
    ValidateQuery,
    #[allow(dead_code)]
    QueryValidated(Result<(), String>),
}

#[derive(Debug, Clone)]
pub enum AuthInput {
    BearerToken(String),
    BasicUser(String),
    BasicPass(String),
    ApiKeyKey(String),
    ApiKeyValue(String),
    ApiKeyLocation(crate::data::auth::ApiKeyLocation),
    DigestUser(String),
    DigestPass(String),
}

#[derive(Debug, Default)]
pub enum RequestStatus {
    #[default]
    Idle,
    Loading,
    Success,
    Error(String),
}

impl Clone for RequestStatus {
    fn clone(&self) -> Self {
        match self {
            RequestStatus::Idle => RequestStatus::Idle,
            RequestStatus::Loading => RequestStatus::Loading,
            RequestStatus::Success => RequestStatus::Success,
            RequestStatus::Error(s) => RequestStatus::Error(s.clone()),
        }
    }
}

fn status_color(status: u16) -> Color {
    match status {
        200..=299 => Color::from_rgb(0.2, 0.7, 0.3),
        300..=399 => Color::from_rgb(0.2, 0.5, 0.8),
        400..=499 => Color::from_rgb(0.8, 0.5, 0.1),
        500..=599 => Color::from_rgb(0.8, 0.2, 0.2),
        _ => Color::from_rgb(0.5, 0.5, 0.5),
    }
}

#[derive(Debug)]
pub struct GraphQLView {
    pub url_input: String,
    pub query_input: text_editor::Content,
    pub variables_input: text_editor::Content,
    pub operation_name: String,
    pub headers_editor: KeyValueEditor,
    pub auth: Auth,
    pub request_config: RequestConfig,
    active_tab: TabId,
    active_response_tab: ResponseTab,
    request_status: RequestStatus,
    pub last_response: Option<GraphQLResponse>,
    pub response_body_editor: text_editor::Content,
    pub status_code: Option<u16>,
    pub content_type: Option<String>,
    pub response_duration: Option<std::time::Duration>,
    pub response_size: Option<u64>,
    pub highlighter_theme: highlighter::Theme,
    pub word_wrap: bool,
    pub query_validation: Option<Result<(), String>>,
}

impl Clone for GraphQLView {
    fn clone(&self) -> Self {
        Self {
            url_input: self.url_input.clone(),
            query_input: text_editor::Content::with_text(&self.query_input.text()),
            variables_input: text_editor::Content::with_text(&self.variables_input.text()),
            operation_name: self.operation_name.clone(),
            headers_editor: self.headers_editor.clone(),
            auth: self.auth.clone(),
            request_config: self.request_config.clone(),
            active_tab: self.active_tab,
            active_response_tab: self.active_response_tab,
            request_status: self.request_status.clone(),
            last_response: self.last_response.clone(),
            response_body_editor: text_editor::Content::with_text(
                &self.response_body_editor.text(),
            ),
            status_code: self.status_code,
            content_type: self.content_type.clone(),
            response_duration: self.response_duration,
            response_size: self.response_size,
            highlighter_theme: self.highlighter_theme,
            word_wrap: self.word_wrap,
            query_validation: self.query_validation.clone(),
        }
    }
}

impl Default for GraphQLView {
    fn default() -> Self {
        Self {
            url_input: "https://countries.trevorblades.com/".to_string(),
            query_input: text_editor::Content::with_text(
                r#"{
  countries {
    code
    name
    emoji
  }
}"#,
            ),
            variables_input: text_editor::Content::new(),
            operation_name: String::new(),
            headers_editor: KeyValueEditor::new("Add Header".to_string()),
            auth: Auth::default(),
            request_config: RequestConfig::default(),
            active_tab: TabId::Query,
            active_response_tab: ResponseTab::Body,
            request_status: RequestStatus::Idle,
            last_response: None,
            response_body_editor: text_editor::Content::new(),
            status_code: None,
            content_type: None,
            response_duration: None,
            response_size: None,
            highlighter_theme: highlighter::Theme::SolarizedDark,
            word_wrap: false,
            query_validation: None,
        }
    }
}

impl GraphQLView {
    pub fn apply_environment(&mut self, env: &crate::persistence::database::Environment) {
        for (key, value) in &env.variables {
            let placeholder = format!("{{{{{}}}}}", key);
            self.url_input = self.url_input.replace(&placeholder, value);

            let new_query = self.query_input.text().replace(&placeholder, value);
            self.query_input = text_editor::Content::with_text(&new_query);

            let new_vars = self.variables_input.text().replace(&placeholder, value);
            self.variables_input = text_editor::Content::with_text(&new_vars);

            self.operation_name = self.operation_name.replace(&placeholder, value);

            for entry in &mut self.headers_editor.entries {
                entry.value = entry.value.replace(&placeholder, value);
            }

            match &mut self.auth {
                Auth::BearerToken(token) => {
                    *token = token.replace(&placeholder, value);
                }
                Auth::Basic { user, pass } => {
                    *user = user.replace(&placeholder, value);
                    *pass = pass.replace(&placeholder, value);
                }
                Auth::ApiKey {
                    key, value: val, ..
                } => {
                    *key = key.replace(&placeholder, value);
                    *val = val.replace(&placeholder, value);
                }
                Auth::Digest { user, pass } => {
                    *user = user.replace(&placeholder, value);
                    *pass = pass.replace(&placeholder, value);
                }
                Auth::OAuth2(config) => {
                    config.auth_url = config.auth_url.replace(&placeholder, value);
                    config.token_url = config.token_url.replace(&placeholder, value);
                    config.device_auth_url = config.device_auth_url.replace(&placeholder, value);
                    config.client_id = config.client_id.replace(&placeholder, value);
                    config.client_secret = config.client_secret.replace(&placeholder, value);
                    config.scopes = config.scopes.replace(&placeholder, value);
                    config.redirect_uri = config.redirect_uri.replace(&placeholder, value);
                    config.access_token = config.access_token.replace(&placeholder, value);
                    config.refresh_token = config.refresh_token.replace(&placeholder, value);
                }
                Auth::None => {}
            }
        }
    }

    pub fn build_request(&self) -> Result<GraphQLRequest, String> {
        let query = self.query_input.text();
        crate::protocols::graphql::validate_query(&query)?;

        let variables = if self.variables_input.text().trim().is_empty() {
            None
        } else {
            Some(crate::protocols::graphql::parse_variables(
                &self.variables_input.text(),
            )?)
        };

        let operation_name = if self.operation_name.trim().is_empty() {
            None
        } else {
            Some(self.operation_name.clone())
        };

        Ok(GraphQLRequest {
            query,
            variables,
            operation_name,
        })
    }

    pub fn build_http_request(&self) -> crate::http_client::request::HttpRequest {
        let graphql_request = self.build_request().unwrap_or_else(|_| GraphQLRequest {
            query: String::new(),
            variables: None,
            operation_name: None,
        });

        let mut headers: Vec<(String, String)> = self
            .headers_editor
            .entries
            .iter()
            .filter(|h| !h.key.is_empty())
            .map(|h| (h.key.clone(), h.value.clone()))
            .collect();

        headers.push(("Content-Type".to_string(), "application/json".to_string()));

        match &self.auth {
            Auth::BearerToken(token) if !token.is_empty() => {
                headers.push(("Authorization".to_string(), format!("Bearer {}", token)));
            }
            Auth::Basic { user, pass } if !user.is_empty() || !pass.is_empty() => {
                let encoded = general_purpose::STANDARD.encode(format!("{}:{}", user, pass));
                headers.push(("Authorization".to_string(), format!("Basic {}", encoded)));
            }
            Auth::ApiKey {
                key,
                value,
                location,
            } if !key.is_empty() => match location {
                crate::data::auth::ApiKeyLocation::Header => {
                    headers.push((key.clone(), value.clone()));
                }
                crate::data::auth::ApiKeyLocation::Query => {
                    // API Key in query not typical for GraphQL, but supported
                }
            },
            _ => {}
        }

        let body = graphql_request.to_json().unwrap_or_default();

        crate::http_client::request::HttpRequest {
            method: "POST".to_string(),
            url: self.url_input.clone(),
            headers,
            body: Some(body),
            config: self.request_config.clone(),
            multipart_fields: vec![],
            auth: Some(self.auth.clone()),
        }
    }

    pub fn update(&mut self, message: Message) {
        match message {
            Message::UrlInputChanged(url) => self.url_input = url,
            Message::QueryChanged(action) => self.query_input.perform(action),
            Message::VariablesChanged(action) => self.variables_input.perform(action),
            Message::OperationNameChanged(name) => self.operation_name = name,
            Message::TabSelected(tab) => self.active_tab = tab,
            Message::ResponseTabSelected(tab) => self.active_response_tab = tab,
            Message::HeadersEditor(msg) => self.headers_editor.update(msg),
            Message::AuthTypeSelected(auth_type) => {
                self.auth = match auth_type {
                    AuthType::NoAuth => Auth::None,
                    AuthType::BearerToken => Auth::BearerToken(String::new()),
                    AuthType::BasicAuth => Auth::Basic {
                        user: String::new(),
                        pass: String::new(),
                    },
                    AuthType::ApiKey => Auth::ApiKey {
                        key: String::new(),
                        value: String::new(),
                        location: crate::data::auth::ApiKeyLocation::Header,
                    },
                    AuthType::Digest => Auth::Digest {
                        user: String::new(),
                        pass: String::new(),
                    },
                    AuthType::OAuth2 => Auth::OAuth2(Box::default()),
                };
            }
            Message::AuthInputChanged(input) => match (&mut self.auth, input) {
                (Auth::BearerToken(token), AuthInput::BearerToken(new_token)) => {
                    *token = new_token;
                }
                (Auth::Basic { user, .. }, AuthInput::BasicUser(new_user)) => {
                    *user = new_user;
                }
                (Auth::Basic { pass, .. }, AuthInput::BasicPass(new_pass)) => {
                    *pass = new_pass;
                }
                (Auth::ApiKey { key, .. }, AuthInput::ApiKeyKey(new_key)) => {
                    *key = new_key;
                }
                (Auth::ApiKey { value, .. }, AuthInput::ApiKeyValue(new_value)) => {
                    *value = new_value;
                }
                (Auth::ApiKey { location, .. }, AuthInput::ApiKeyLocation(new_location)) => {
                    *location = new_location;
                }
                (Auth::Digest { user, .. }, AuthInput::DigestUser(new_user)) => {
                    *user = new_user;
                }
                (Auth::Digest { pass, .. }, AuthInput::DigestPass(new_pass)) => {
                    *pass = new_pass;
                }
                _ => {}
            },
            Message::SendRequest => {}
            Message::SetLoading => {
                self.request_status = RequestStatus::Loading;
                self.last_response = None;
                self.response_body_editor = text_editor::Content::new();
                self.status_code = None;
                self.content_type = None;
                self.response_duration = None;
                self.response_size = None;
            }
            Message::ResponseReceived(result) => match result {
                Ok((response, status, headers, duration, size)) => {
                    self.status_code = Some(status);
                    self.response_duration = Some(duration);
                    self.response_size = Some(size);
                    let ct = headers
                        .iter()
                        .find(|(k, _)| k.eq_ignore_ascii_case("content-type"))
                        .map(|(_, v)| v.clone())
                        .unwrap_or_else(|| "application/json".to_string());
                    self.content_type = Some(ct);

                    let formatted = crate::protocols::graphql::format_response(&response);
                    self.response_body_editor = text_editor::Content::with_text(&formatted);
                    self.last_response = Some(response);
                    self.request_status = RequestStatus::Success;
                }
                Err(e) => {
                    self.request_status = RequestStatus::Error(format!("Error: {}", e));
                    self.last_response = None;
                    self.response_body_editor = text_editor::Content::new();
                    self.status_code = None;
                    self.content_type = None;
                    self.response_duration = None;
                    self.response_size = None;
                }
            },
            Message::CopyResponse => {
                let text = self.response_body_editor.text();
                if !text.is_empty() {
                    if let Ok(mut clipboard) = arboard::Clipboard::new() {
                        let _ = clipboard.set_text(text);
                    }
                }
            }
            Message::CopyHeaders => {
                if let Some(response) = &self.last_response {
                    if let Ok(json) = serde_json::to_string_pretty(response) {
                        if let Ok(mut clipboard) = arboard::Clipboard::new() {
                            let _ = clipboard.set_text(&json);
                        }
                    }
                }
            }
            Message::CopyBody => {
                let text = self.response_body_editor.text();
                if !text.is_empty() {
                    if let Ok(mut clipboard) = arboard::Clipboard::new() {
                        let _ = clipboard.set_text(text);
                    }
                }
            }
            Message::CopySelection => {
                if let Some(selection) = self.response_body_editor.selection() {
                    if let Ok(mut clipboard) = arboard::Clipboard::new() {
                        let _ = clipboard.set_text(selection);
                    }
                }
            }
            Message::ResponseContentChanged(action) => {
                self.response_body_editor.perform(action);
            }
            Message::ToggleWordWrap => {
                self.word_wrap = !self.word_wrap;
            }
            Message::ValidateQuery => {
                self.query_validation = Some(crate::protocols::graphql::validate_query(
                    &self.query_input.text(),
                ));
            }
            Message::QueryValidated(result) => {
                self.query_validation = Some(result);
            }
        }
    }

    pub fn view(&self) -> Element<'_, Message, Theme, Renderer> {
        let url_bar = row![
            text("POST").size(14).color(method_color("POST")),
            text_input("GraphQL endpoint URL", &self.url_input)
                .on_input(Message::UrlInputChanged)
                .padding(10),
            button(row![lucide::send().size(14), text(" Send")].spacing(4))
                .on_press(Message::SendRequest),
        ]
        .spacing(10)
        .padding(10)
        .align_y(Alignment::Center);

        let query_tab = {
            let editor = text_editor(&self.query_input)
                .on_action(Message::QueryChanged)
                .highlight("graphql", self.highlighter_theme);
            let context_menu = ContextMenu::new(scrollable(editor), || {
                column![
                    button(row![lucide::copy().size(12), text(" Copy Query")].spacing(4))
                        .on_press(Message::CopyBody),
                    button(row![lucide::check().size(12), text(" Validate")].spacing(4))
                        .on_press(Message::ValidateQuery),
                ]
                .into()
            });
            container(context_menu)
                .padding(5)
                .width(Length::Fill)
                .height(Length::Fill)
        };

        let variables_tab = {
            let editor = text_editor(&self.variables_input)
                .on_action(Message::VariablesChanged)
                .highlight("json", self.highlighter_theme);
            container(scrollable(editor))
                .padding(5)
                .width(Length::Fill)
                .height(Length::Fill)
        };

        let headers_tab = container(self.headers_editor.view().map(Message::HeadersEditor))
            .padding(10)
            .width(Length::Fill)
            .height(Length::Fill);

        let auth_tab = container(self.create_auth_tab_content())
            .padding(10)
            .width(Length::Fill)
            .height(Length::Fill);

        let tabs = Tabs::new(Message::TabSelected)
            .push(TabId::Query, TabLabel::Text("Query".to_string()), query_tab)
            .push(
                TabId::Variables,
                TabLabel::Text("Variables".to_string()),
                variables_tab,
            )
            .push(
                TabId::Headers,
                TabLabel::Text("Headers".to_string()),
                headers_tab,
            )
            .push(
                TabId::Authorization,
                TabLabel::Text("Authorization".to_string()),
                auth_tab,
            )
            .set_active_tab(&self.active_tab)
            .width(Length::Fill)
            .height(Length::Fixed(250.0));

        let response_area: Element<Message> = match &self.request_status {
            RequestStatus::Idle => {
                let placeholder = if let Some(Err(e)) = &self.query_validation {
                    container(
                        column![
                            text("Enter query and send request.").size(14),
                            text(e.clone())
                                .size(12)
                                .color(Color::from_rgb(0.8, 0.2, 0.2)),
                        ]
                        .spacing(5),
                    )
                } else {
                    container(text("Enter query and send request.").size(14))
                };
                placeholder
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .align_x(Alignment::Center)
                    .align_y(Alignment::Center)
                    .into()
            }
            RequestStatus::Loading => container(text("Loading..."))
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(Alignment::Center)
                .align_y(Alignment::Center)
                .into(),
            RequestStatus::Success => {
                let response_tabs = Tabs::new(Message::ResponseTabSelected)
                    .push(ResponseTab::Body, TabLabel::Text("Body".to_string()), {
                        if self.word_wrap {
                            let body_text = self.response_body_editor.text();
                            let wrapped = text(body_text).size(13).font(iced::Font::MONOSPACE);
                            let context_menu = ContextMenu::new(scrollable(wrapped), || {
                                column![button(
                                    row![lucide::copy().size(12), text(" Copy Body")].spacing(4)
                                )
                                .on_press(Message::CopyBody)]
                                .into()
                            });
                            container(context_menu)
                        } else {
                            let editor = text_editor(&self.response_body_editor)
                                .on_action(Message::ResponseContentChanged)
                                .highlight("json", self.highlighter_theme);
                            let context_menu = ContextMenu::new(scrollable(editor), || {
                                column![
                                    button(
                                        row![lucide::copy().size(12), text(" Copy Selection")]
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
                            container(context_menu)
                        }
                    })
                    .push(
                        ResponseTab::Headers,
                        TabLabel::Text("Data".to_string()),
                        self.create_response_data_view(),
                    )
                    .set_active_tab(&self.active_response_tab)
                    .width(Length::Fill)
                    .height(Length::Fill);

                container(response_tabs)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .into()
            }
            RequestStatus::Error(error_message) => {
                container(text(format!("Error: {}", error_message)))
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .align_x(Alignment::Center)
                    .align_y(Alignment::Center)
                    .into()
            }
        };

        let status_text = if let Some(status) = self.status_code {
            let color = status_color(status);
            text(format!("  {}  ", status)).size(14).color(color)
        } else {
            text(String::new()).size(14)
        };

        let duration_text = text(format!(
            "{}ms",
            self.response_duration
                .map(|d| d.as_millis().to_string())
                .unwrap_or_else(|| "N/A".to_string())
        ))
        .size(14);

        let size_text = text(
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
        .size(14);

        let method_label = text("GraphQL")
            .size(14)
            .color(Color::from_rgb(0.8, 0.3, 0.6));

        let copy_button = if matches!(
            self.request_status,
            RequestStatus::Success | RequestStatus::Error(_)
        ) {
            Element::from(
                button(row![lucide::copy().size(14), text(" Copy")].spacing(4))
                    .on_press(Message::CopyResponse),
            )
        } else {
            Element::from(column![])
        };

        let wrap_toggle: Element<'_, Message, Theme, Renderer> =
            if matches!(self.request_status, RequestStatus::Success) {
                Element::from(
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
                    .on_press(Message::ToggleWordWrap),
                )
            } else {
                Element::from(column![])
            };

        let validation_indicator: Element<'_, Message, Theme, Renderer> =
            match &self.query_validation {
                Some(Ok(())) => text("Valid")
                    .size(12)
                    .color(Color::from_rgb(0.2, 0.7, 0.3))
                    .into(),
                Some(Err(e)) => text(e.clone())
                    .size(12)
                    .color(Color::from_rgb(0.8, 0.2, 0.2))
                    .into(),
                None => column![].into(),
            };

        let operation_name_display: Element<'_, Message, Theme, Renderer> =
            if self.operation_name.is_empty() {
                column![].into()
            } else {
                text(format!("Op: {}", self.operation_name))
                    .size(12)
                    .color(Color::from_rgb(0.5, 0.5, 0.5))
                    .into()
            };

        let main_column = column![
            url_bar,
            row![
                text_input("Operation name (optional)", &self.operation_name)
                    .on_input(Message::OperationNameChanged)
                    .padding(8)
                    .width(Length::Fixed(200.0)),
                validation_indicator,
                operation_name_display,
            ]
            .spacing(10)
            .padding(10)
            .align_y(Alignment::Center),
            tabs,
            rule::horizontal(10),
            column![
                row![
                    method_label,
                    status_text,
                    duration_text,
                    text(" | ").size(14),
                    size_text,
                    row![copy_button, wrap_toggle].align_y(Alignment::Center),
                ]
                .spacing(10)
                .padding(10)
                .align_y(Alignment::Center),
                response_area,
            ]
            .height(Length::Fill),
        ]
        .align_x(Alignment::Center);

        scrollable(main_column).into()
    }

    fn create_auth_tab_content(&self) -> Element<'_, Message, Theme, Renderer> {
        let current_auth_type = self.auth.auth_type();

        let auth_type_selector = pick_list(
            &AuthType::ALL[..],
            Some(current_auth_type),
            Message::AuthTypeSelected,
        )
        .padding(10);

        let auth_inputs = match &self.auth {
            Auth::BearerToken(token) => column![text_input("Bearer Token", token)
                .on_input(|t| Message::AuthInputChanged(AuthInput::BearerToken(t)))
                .padding(10)
                .secure(true),]
            .spacing(10),
            Auth::Basic { user, pass } => column![
                text_input("Username", user)
                    .on_input(|u| Message::AuthInputChanged(AuthInput::BasicUser(u)))
                    .padding(10),
                text_input("Password", pass)
                    .on_input(|p| Message::AuthInputChanged(AuthInput::BasicPass(p)))
                    .padding(10)
                    .secure(true),
            ]
            .spacing(10),
            Auth::ApiKey {
                key,
                value,
                location,
            } => column![
                text_input("Key Name", key)
                    .on_input(|k| Message::AuthInputChanged(AuthInput::ApiKeyKey(k)))
                    .padding(10),
                text_input("Value", value)
                    .on_input(|v| Message::AuthInputChanged(AuthInput::ApiKeyValue(v)))
                    .padding(10),
                pick_list(
                    &crate::data::auth::ApiKeyLocation::ALL[..],
                    Some(*location),
                    |loc| Message::AuthInputChanged(AuthInput::ApiKeyLocation(loc)),
                )
                .padding(10),
            ]
            .spacing(10),
            Auth::Digest { user, pass } => column![
                text("Digest Authentication").size(14),
                text_input("Username", user)
                    .on_input(|u| Message::AuthInputChanged(AuthInput::DigestUser(u)))
                    .padding(10),
                text_input("Password", pass)
                    .on_input(|p| Message::AuthInputChanged(AuthInput::DigestPass(p)))
                    .padding(10)
                    .secure(true),
            ]
            .spacing(10),
            Auth::OAuth2(_) => column![
                text("OAuth2 not fully supported for GraphQL yet").size(12),
                text("Use Bearer token or manually configure").size(12),
            ]
            .spacing(10),
            Auth::None => column![text("No authentication configured").size(14)].spacing(10),
        };

        column![
            text("Authentication Type:").size(14),
            auth_type_selector,
            auth_inputs,
        ]
        .spacing(10)
        .into()
    }

    fn create_response_data_view(&self) -> Element<'_, Message, Theme, Renderer> {
        if let Some(response) = &self.last_response {
            let mut items = column![].spacing(8);

            if let Some(data) = &response.data {
                let data_str = serde_json::to_string_pretty(data).unwrap_or_default();
                items = items.push(
                    row![text("Data:").size(14).color(Color::from_rgb(0.5, 0.5, 0.5)),].spacing(8),
                );
                items = items.push(text(data_str).size(13).font(iced::Font::MONOSPACE));
            }

            if !response.errors.is_empty() {
                items = items.push(rule::horizontal(5));
                items = items.push(
                    text(format!("Errors ({}):", response.errors.len()))
                        .size(14)
                        .color(Color::from_rgb(0.8, 0.3, 0.3)),
                );
                for (i, err) in response.errors.iter().enumerate() {
                    items = items.push(
                        text(format!("{}. {}", i + 1, err.message))
                            .size(13)
                            .color(Color::from_rgb(0.8, 0.3, 0.3)),
                    );
                }
            }

            container(scrollable(items))
                .padding(10)
                .width(Length::Fill)
                .height(Length::Fill)
                .into()
        } else {
            container(text("No response data available."))
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(Alignment::Center)
                .align_y(Alignment::Center)
                .into()
        }
    }
}
