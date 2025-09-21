use crate::data::auth::{Auth, AuthType};
use crate::ui::components::key_value_editor::{self, KeyValueEditor};
use base64::{engine::general_purpose, Engine as _};
use bytes::Bytes;
use iced::widget::image::{Handle, Image};
use iced::{
    widget::{
        button, column, container, pick_list, row, scrollable, text, text_editor, text_input, Rule,
    },
    Alignment, Element, Length, Renderer, Theme,
};
use iced_aw::{ContextMenu, TabLabel, Tabs};
use std::time::Duration;

const LOGO_BG_BYTES: &[u8] = include_bytes!("../../../assets/logo-bg.png");

static HTTP_METHODS: [&str; 7] = ["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContentType {
    Json,
    Text,
    Html,
    Xml,
}

impl ContentType {
    pub const ALL: [ContentType; 4] = [
        ContentType::Json,
        ContentType::Text,
        ContentType::Html,
        ContentType::Xml,
    ];
}

impl std::fmt::Display for ContentType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                ContentType::Json => "JSON",
                ContentType::Text => "Text",
                ContentType::Html => "HTML",
                ContentType::Xml => "XML",
            }
        )
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    UrlInputChanged(String),
    MethodSelected(&'static str),
    TabSelected(TabId),
    AuthTypeSelected(AuthType),
    AuthInputChanged(AuthInput),
    HeadersEditor(key_value_editor::Message),
    ParamsEditor(key_value_editor::Message),
    BodyInputChanged(text_editor::Action),
    RequestContentTypeSelected(ContentType),
    SendRequest(crate::http_client::request::HttpRequest),
    SetLoading,
    ResponseReceived(Result<crate::http_client::response::HttpResponse, String>),
    CopyResponse,
    ResponseContentChanged(text_editor::Action),
    CopySelection,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum TabId {
    #[default]
    Body,
    Headers,
    Params,
    Authorization,
}

#[derive(Debug, Clone)]
pub enum AuthInput {
    BearerToken(String),
    BasicUser(String),
    BasicPass(String),
}

#[derive(Debug, Default)]
pub enum RequestStatus {
    #[default]
    Idle,
    Loading,
    Success(text_editor::Content),
    Error(String),
}

impl Clone for RequestStatus {
    fn clone(&self) -> Self {
        match self {
            RequestStatus::Idle => RequestStatus::Idle,
            RequestStatus::Loading => RequestStatus::Loading,
            RequestStatus::Success(content) => {
                RequestStatus::Success(text_editor::Content::with_text(&content.text()))
            }
            RequestStatus::Error(s) => RequestStatus::Error(s.clone()),
        }
    }
}

#[derive(Debug)]
pub struct HttpRequestView {
    pub url_input: String,
    pub method: &'static str,
    pub body_input: text_editor::Content,
    pub auth: Auth,
    pub headers_editor: KeyValueEditor,
    pub params_editor: KeyValueEditor,
    active_tab: TabId,
    request_status: RequestStatus,
    pub status_code: Option<u16>,
    pub content_type: Option<String>,
    pub response_duration: Option<Duration>,
    pub response_size: Option<u64>,
    pub request_content_type: ContentType,
}

impl Clone for HttpRequestView {
    fn clone(&self) -> Self {
        Self {
            url_input: self.url_input.clone(),
            method: self.method,
            body_input: text_editor::Content::with_text(&self.body_input.text()),
            auth: self.auth.clone(),
            headers_editor: self.headers_editor.clone(),
            params_editor: self.params_editor.clone(),
            active_tab: self.active_tab.clone(),
            request_status: self.request_status.clone(),
            status_code: self.status_code,
            content_type: self.content_type.clone(),
            response_duration: self.response_duration,
            response_size: self.response_size,
            request_content_type: self.request_content_type,
        }
    }
}

impl Default for HttpRequestView {
    fn default() -> Self {
        Self {
            url_input: "https://jsonplaceholder.typicode.com/todos/1".to_string(),
            method: "GET",
            body_input: text_editor::Content::new(),
            auth: Auth::default(),
            headers_editor: KeyValueEditor::new("Add Header".to_string()),
            params_editor: KeyValueEditor::new("Add Param".to_string()),
            active_tab: TabId::Body,
            request_status: RequestStatus::Idle,
            status_code: None,
            content_type: None,
            response_duration: None,
            response_size: None,
            request_content_type: ContentType::Json,
        }
    }
}

impl HttpRequestView {
    pub fn build_request(&self) -> crate::http_client::request::HttpRequest {
        let params: Vec<(String, String)> = self
            .params_editor
            .entries
            .iter()
            .filter(|p| !p.key.is_empty())
            .map(|p| (p.key.clone(), p.value.clone()))
            .collect();

        let query_string = params
            .iter()
            .map(|(k, v)| format!("{}={}", urlencoding::encode(k), urlencoding::encode(v)))
            .collect::<Vec<String>>()
            .join("&");

        let final_url = if query_string.is_empty() {
            self.url_input.clone()
        } else if self.url_input.contains('?') {
            format!("{}&{}", self.url_input, query_string)
        } else {
            format!("{}?{}", self.url_input, query_string)
        };

        let mut headers: Vec<(String, String)> = self
            .headers_editor
            .entries
            .iter()
            .filter(|h| !h.key.is_empty())
            .map(|h| (h.key.clone(), h.value.clone()))
            .collect();

        match &self.auth {
            Auth::BearerToken(token) => {
                if !token.is_empty() {
                    headers.push(("Authorization".to_string(), format!("Bearer {}", token)));
                }
            }
            Auth::Basic { user, pass } => {
                if !user.is_empty() || !pass.is_empty() {
                    let encoded = general_purpose::STANDARD.encode(format!("{}:{}", user, pass));
                    headers.push(("Authorization".to_string(), format!("Basic {}", encoded)));
                }
            }
            _ => {}
        }
        
        let body = if self.body_input.text().is_empty() {
            None
        } else {
            Some(self.body_input.text())
        };

        if body.is_some() {
            let content_type_str = match self.request_content_type {
                ContentType::Json => "application/json",
                ContentType::Text => "text/plain",
                ContentType::Html => "text/html",
                ContentType::Xml => "application/xml",
            };
            headers.push(("Content-Type".to_string(), content_type_str.to_string()));
        }

        crate::http_client::request::HttpRequest {
            method: self.method.to_string(),
            url: final_url,
            headers,
            body,
        }
    }

    pub fn update(&mut self, message: Message) {
        match message {
            Message::UrlInputChanged(url) => self.url_input = url,
            Message::MethodSelected(method) => self.method = method,
            Message::TabSelected(tab_id) => self.active_tab = tab_id,
            Message::AuthTypeSelected(auth_type) => {
                self.auth = match auth_type {
                    AuthType::NoAuth => Auth::None,
                    AuthType::BearerToken => Auth::BearerToken(String::new()),
                    AuthType::BasicAuth => Auth::Basic {
                        user: String::new(),
                        pass: String::new(),
                    },
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
                _ => {}
            },
            Message::HeadersEditor(msg) => self.headers_editor.update(msg),
            Message::ParamsEditor(msg) => self.params_editor.update(msg),
            Message::BodyInputChanged(action) => self.body_input.perform(action),
            Message::RequestContentTypeSelected(content_type) => {
                self.request_content_type = content_type
            }
            Message::SendRequest(_) => {}
            Message::SetLoading => {
                self.request_status = RequestStatus::Loading;
                self.status_code = None;
                self.content_type = None;
                self.response_duration = None;
                self.response_size = None;
            }
            Message::ResponseReceived(result) => match result {
                Ok(response) => {
                    self.status_code = Some(response.status);
                    self.response_duration = Some(response.duration);
                    self.response_size = Some(response.size);
                    let content_type = response
                        .headers
                        .iter()
                        .find(|(k, _)| k.eq_ignore_ascii_case("content-type"))
                        .map(|(_, v)| v.clone())
                        .unwrap_or_else(|| "unknown".to_string());
                    self.content_type = Some(content_type.clone());

                    let formatted_body = if content_type.contains("application/json") {
                        match serde_json::from_str::<serde_json::Value>(&response.body) {
                            Ok(json_value) => serde_json::to_string_pretty(&json_value)
                                .unwrap_or_else(|_| response.body.clone()),
                            Err(_) => response.body.clone(),
                        }
                    } else {
                        response.body.clone()
                    };

                    let response_text = format!(
                        r#"Headers: {headers:#?}

Body: {body}

--------------------
URL: {url}
Method: {method}"#,
                        headers = response.headers,
                        body = formatted_body,
                        url = response.url,
                        method = response.method,
                    );
                    self.request_status =
                        RequestStatus::Success(text_editor::Content::with_text(&response_text));
                }
                Err(e) => {
                    self.request_status = RequestStatus::Error(format!("Error: {}", e));
                    self.status_code = None;
                    self.content_type = None;
                    self.response_duration = None;
                    self.response_size = None;
                }
            },
            Message::CopyResponse => {
                let text_to_copy = match &self.request_status {
                    RequestStatus::Success(content) => Some(content.text()),
                    RequestStatus::Error(error_message) => Some(error_message.clone()),
                    _ => None,
                };

                if let Some(text) = text_to_copy {
                    let mut clipboard = arboard::Clipboard::new().unwrap();
                    clipboard.set_text(text).unwrap();
                }
            }
            Message::ResponseContentChanged(action) => {
                if let RequestStatus::Success(content) = &mut self.request_status {
                    content.perform(action);
                }
            }
            Message::CopySelection => {
                if let RequestStatus::Success(content) = &self.request_status {
                    if let Some(selection) = content.selection() {
                        let mut clipboard = arboard::Clipboard::new().unwrap();
                        clipboard.set_text(selection).unwrap();
                    }
                }
            }
        }
    }

    pub fn view(&self) -> Element<'_, Message, Theme, Renderer> {
        let auth_tab_content = self.create_auth_tab_content();
        let body_tab_content = self.create_body_tab_content();

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
            .set_active_tab(&self.active_tab)
            .width(Length::Fill);

        let response_area: Element<Message> = match &self.request_status {
            RequestStatus::Idle => container(text("Enter URL and send request."))
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(Alignment::Center)
                .align_y(Alignment::Center)
                .into(),
            RequestStatus::Loading => container(text("Loading..."))
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(Alignment::Center)
                .align_y(Alignment::Center)
                .into(),
            RequestStatus::Success(content) => {
                let editor = text_editor(content).on_action(Message::ResponseContentChanged);

                let context_menu = ContextMenu::new(scrollable(editor), || {
                    button("Copy Selection")
                        .on_press(Message::CopySelection)
                        .into()
                });

                container(context_menu)
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

        let copy_button = if matches!(
            self.request_status,
            RequestStatus::Success(_) | RequestStatus::Error(_)
        ) {
            Element::from(button("Copy").on_press(Message::CopyResponse))
        } else {
            Element::from(column![])
        };

        let status_code_text = text(format!(
            "Status: {}",
            self.status_code
                .map(|s| s.to_string())
                .unwrap_or_else(|| "N/A".to_string())
        ))
        .size(16);
        let content_type_text = text(format!(
            "Content-Type: {}",
            self.content_type.as_deref().unwrap_or("N/A")
        ))
        .size(16);
        let duration_text = text(format!(
            "Time: {}ms",
            self.response_duration
                .map(|d| d.as_millis().to_string())
                .unwrap_or_else(|| "N/A".to_string())
        ))
        .size(16);
        let size_text = text(format!(
            "Size: {} B",
            self.response_size
                .map(|s| s.to_string())
                .unwrap_or_else(|| "N/A".to_string())
        ))
        .size(16);

        let main_column = column![
            Image::new(Handle::from_bytes(Bytes::from_static(LOGO_BG_BYTES)))
                .width(Length::Fixed(100.0))
                .height(Length::Fixed(100.0)),
            row![
                pick_list(
                    &HTTP_METHODS[..],
                    Some(self.method),
                    Message::MethodSelected
                )
                .padding(10),
                text_input("URL", &self.url_input)
                    .on_input(Message::UrlInputChanged)
                    .padding(10),
                button("Send").on_press(Message::SendRequest(self.build_request()))
            ]
            .spacing(10)
            .padding(10),
            tabs.height(Length::Fixed(250.0)),
            Rule::horizontal(10),
            column![
                row![
                    status_code_text,
                    content_type_text,
                    duration_text,
                    size_text,
                ]
                .spacing(20)
                .padding(10),
                row![response_area, copy_button,]
                    .spacing(10)
                    .padding(10)
                    .height(Length::Fill),
            ]
            .height(Length::Fill),
        ]
        .align_x(Alignment::Center);

        main_column.into()
    }

    fn create_auth_tab_content(&self) -> Element<'_, Message, Theme, Renderer> {
        let current_auth_type = match self.auth {
            Auth::None => AuthType::NoAuth,
            Auth::BearerToken(_) => AuthType::BearerToken,
            Auth::Basic { .. } => AuthType::BasicAuth,
        };

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
            Auth::None => column![text("No authentication required.").size(14),].spacing(10),
        };

        container(
            column![
                text("Authentication Type").size(16),
                auth_type_selector,
                auth_inputs
            ]
            .spacing(15)
            .padding(20),
        )
        .into()
    }

    fn create_body_tab_content(&self) -> Element<'_, Message, Theme, Renderer> {
        let content_type_selector = pick_list(
            &ContentType::ALL[..],
            Some(self.request_content_type),
            Message::RequestContentTypeSelected,
        )
        .padding(10);

        let body_editor = text_editor(&self.body_input)
            .on_action(Message::BodyInputChanged)
            .height(Length::Fill);

        container(
            column![
                row![text("Content-Type:").size(16), content_type_selector].spacing(10),
                body_editor
            ]
            .spacing(15)
            .padding(10),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }
}
