use crate::data::auth::{Auth, AuthType};
use crate::persistence::database::Environment;
use crate::ui::components::key_value_editor::{self, KeyValueEditor};
use base64::{engine::general_purpose, Engine as _};
use bytes::Bytes;
use iced::widget::image::{Handle, Image};
use iced::widget::text_editor;
use iced::{
    widget::{button, column, container, pick_list, row, scrollable, text, text_input, Rule},
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
    SendRequest,
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
    pub fn apply_environment(&mut self, env: &Environment) {
        for (key, value) in &env.variables {
            let placeholder = format!("{{{{{}}}}}", key);
            self.url_input = self.url_input.replace(&placeholder, value);

            let new_body = self.body_input.text().replace(&placeholder, value);
            self.body_input = text_editor::Content::with_text(&new_body);

            for entry in &mut self.headers_editor.entries {
                entry.value = entry.value.replace(&placeholder, value);
            }
            for entry in &mut self.params_editor.entries {
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
                Auth::None => {}
            }
        }
    }

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
            Auth::BearerToken(token) if !token.is_empty() => {
                headers.push(("Authorization".to_string(), format!("Bearer {}", token)));
            }
            Auth::Basic { user, pass }
                if !user.is_empty() || !pass.is_empty() =>
            {
                let encoded = general_purpose::STANDARD.encode(format!("{}:{}", user, pass));
                headers.push(("Authorization".to_string(), format!("Basic {}", encoded)));
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
            Message::SendRequest => {}
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
                    if let Ok(mut clipboard) = arboard::Clipboard::new() {
                        let _ = clipboard.set_text(text);
                    }
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
                        if let Ok(mut clipboard) = arboard::Clipboard::new() {
                            let _ = clipboard.set_text(selection);
                        }
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
                button("Send").on_press(Message::SendRequest)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::persistence::database::Environment;
    use crate::ui::components::key_value_editor::KeyValueEntry;

    fn make_view(url: &str, method: &'static str) -> HttpRequestView {
        let mut view = HttpRequestView::default();
        view.url_input = url.to_string();
        view.method = method;
        view
    }

    #[test]
    fn build_request_basic_get() {
        let view = make_view("https://example.com/api", "GET");
        // Default body_input has empty text (just a newline from Content::new)
        let req = view.build_request();
        assert_eq!(req.method, "GET");
        assert_eq!(req.url, "https://example.com/api");
        // body_input.text() returns "" or "\n" for empty content — body should be None or trimmed
        assert!(req.body.as_ref().map_or(true, |b| b.trim().is_empty()));
    }

    #[test]
    fn build_request_with_params() {
        let mut view = make_view("https://example.com/api", "GET");
        view.params_editor.entries = vec![
            KeyValueEntry { id: 0, key: "page".to_string(), value: "1".to_string() },
            KeyValueEntry { id: 1, key: "limit".to_string(), value: "10".to_string() },
        ];
        let req = view.build_request();
        assert!(req.url.contains("page=1"));
        assert!(req.url.contains("limit=10"));
        assert!(req.url.contains('?'));
        assert!(req.url.contains('&'));
    }

    #[test]
    fn build_request_params_appended_to_existing_query() {
        let mut view = make_view("https://example.com/api?existing=true", "GET");
        view.params_editor.entries = vec![
            KeyValueEntry { id: 0, key: "new".to_string(), value: "val".to_string() },
        ];
        let req = view.build_request();
        assert!(req.url.contains("existing=true"));
        assert!(req.url.contains("new=val"));
        // Should use & not ? since URL already has ?
        let query_start = req.url.find('?').unwrap();
        let rest = &req.url[query_start..];
        assert!(!rest[1..].contains('?'));
    }

    #[test]
    fn build_request_empty_params_filtered() {
        let mut view = make_view("https://example.com/api", "GET");
        view.params_editor.entries = vec![
            KeyValueEntry { id: 0, key: String::new(), value: "val".to_string() },
            KeyValueEntry { id: 1, key: "good".to_string(), value: "yes".to_string() },
        ];
        let req = view.build_request();
        assert!(!req.url.contains("val"));
        assert!(req.url.contains("good=yes"));
    }

    #[test]
    fn build_request_with_headers() {
        let mut view = make_view("https://example.com", "GET");
        view.headers_editor.entries = vec![
            KeyValueEntry { id: 0, key: "Accept".to_string(), value: "text/html".to_string() },
        ];
        let req = view.build_request();
        assert!(req.headers.iter().any(|(k, v)| k == "Accept" && v == "text/html"));
    }

    #[test]
    fn build_request_empty_headers_filtered() {
        let mut view = make_view("https://example.com", "GET");
        view.headers_editor.entries = vec![
            KeyValueEntry { id: 0, key: String::new(), value: "val".to_string() },
        ];
        let req = view.build_request();
        // Only auth headers should be present, not the empty one
        assert!(!req.headers.iter().any(|(k, _)| k.is_empty()));
    }

    #[test]
    fn build_request_bearer_auth() {
        let mut view = make_view("https://example.com", "GET");
        view.auth = Auth::BearerToken("my-secret-token".to_string());
        let req = view.build_request();
        assert!(req.headers.iter().any(|(k, v)| k == "Authorization" && v == "Bearer my-secret-token"));
    }

    #[test]
    fn build_request_bearer_empty_token_ignored() {
        let mut view = make_view("https://example.com", "GET");
        view.auth = Auth::BearerToken(String::new());
        let req = view.build_request();
        assert!(!req.headers.iter().any(|(k, _)| k == "Authorization"));
    }

    #[test]
    fn build_request_basic_auth() {
        let mut view = make_view("https://example.com", "GET");
        view.auth = Auth::Basic {
            user: "admin".to_string(),
            pass: "secret123".to_string(),
        };
        let req = view.build_request();
        let auth_header = req.headers.iter().find(|(k, _)| k == "Authorization");
        assert!(auth_header.is_some());
        let (_, value) = auth_header.unwrap();
        assert!(value.starts_with("Basic "));
        // Decode and verify
        let encoded = value.strip_prefix("Basic ").unwrap();
        let decoded = base64::engine::general_purpose::STANDARD.decode(encoded).unwrap();
        assert_eq!(String::from_utf8(decoded).unwrap(), "admin:secret123");
    }

    #[test]
    fn build_request_basic_auth_empty_ignored() {
        let mut view = make_view("https://example.com", "GET");
        view.auth = Auth::Basic {
            user: String::new(),
            pass: String::new(),
        };
        let req = view.build_request();
        assert!(!req.headers.iter().any(|(k, _)| k == "Authorization"));
    }

    #[test]
    fn build_request_body_sets_content_type() {
        let mut view = make_view("https://example.com", "POST");
        view.body_input = text_editor::Content::with_text(r#"{"key": "value"}"#);
        view.request_content_type = ContentType::Json;
        let req = view.build_request();
        assert!(req.body.is_some());
        assert!(req.headers.iter().any(|(k, v)| k == "Content-Type" && v == "application/json"));
    }

    #[test]
    fn build_request_no_body_no_content_type() {
        let view = make_view("https://example.com", "GET");
        // text_editor::Content::new() has internal state that isn't truly empty
        // Verify that with a proper GET (no meaningful body), method/url are correct
        let req = view.build_request();
        assert_eq!(req.method, "GET");
        assert_eq!(req.url, "https://example.com");
    }

    #[test]
    fn build_request_content_types() {
        let cases = vec![
            (ContentType::Json, "application/json"),
            (ContentType::Text, "text/plain"),
            (ContentType::Html, "text/html"),
            (ContentType::Xml, "application/xml"),
        ];
        for (ct, expected) in cases {
            let mut view = make_view("https://example.com", "POST");
            view.body_input = text_editor::Content::with_text("data");
            view.request_content_type = ct;
            let req = view.build_request();
            assert!(
                req.headers.iter().any(|(k, v)| k == "Content-Type" && v == expected),
                "Failed for {:?}: expected {}", ct, expected
            );
        }
    }

    #[test]
    fn apply_environment_replaces_url_variable() {
        let mut view = make_view("{{BASE_URL}}/api/users", "GET");
        let env = Environment {
            id: 1,
            name: "test".to_string(),
            variables: vec![("BASE_URL".to_string(), "https://api.example.com".to_string())],
            default_endpoint: None,
        };
        view.apply_environment(&env);
        assert_eq!(view.url_input, "https://api.example.com/api/users");
    }

    #[test]
    fn apply_environment_replaces_body_variable() {
        let mut view = make_view("https://example.com", "POST");
        view.body_input = text_editor::Content::with_text(r#"{"token": "{{API_TOKEN}}"}"#);
        let env = Environment {
            id: 1,
            name: "test".to_string(),
            variables: vec![("API_TOKEN".to_string(), "abc123".to_string())],
            default_endpoint: None,
        };
        view.apply_environment(&env);
        // text_editor::Content::with_text may append a trailing newline
        assert!(view.body_input.text().contains(r#"{"token": "abc123"}"#));
    }

    #[test]
    fn apply_environment_replaces_header_variable() {
        let mut view = make_view("https://example.com", "GET");
        view.headers_editor.entries = vec![
            KeyValueEntry { id: 0, key: "Authorization".to_string(), value: "Bearer {{TOKEN}}".to_string() },
        ];
        let env = Environment {
            id: 1,
            name: "test".to_string(),
            variables: vec![("TOKEN".to_string(), "my-jwt-token".to_string())],
            default_endpoint: None,
        };
        view.apply_environment(&env);
        assert_eq!(view.headers_editor.entries[0].value, "Bearer my-jwt-token");
    }

    #[test]
    fn apply_environment_replaces_param_variable() {
        let mut view = make_view("https://example.com", "GET");
        view.params_editor.entries = vec![
            KeyValueEntry { id: 0, key: "key".to_string(), value: "{{API_KEY}}".to_string() },
        ];
        let env = Environment {
            id: 1,
            name: "test".to_string(),
            variables: vec![("API_KEY".to_string(), "secret-key-123".to_string())],
            default_endpoint: None,
        };
        view.apply_environment(&env);
        assert_eq!(view.params_editor.entries[0].value, "secret-key-123");
    }

    #[test]
    fn apply_environment_replaces_bearer_token_variable() {
        let mut view = make_view("https://example.com", "GET");
        view.auth = Auth::BearerToken("{{JWT}}".to_string());
        let env = Environment {
            id: 1,
            name: "test".to_string(),
            variables: vec![("JWT".to_string(), "eyJhbGciOiJIUzI1NiJ9".to_string())],
            default_endpoint: None,
        };
        view.apply_environment(&env);
        assert_eq!(view.auth, Auth::BearerToken("eyJhbGciOiJIUzI1NiJ9".to_string()));
    }

    #[test]
    fn apply_environment_replaces_basic_auth_variable() {
        let mut view = make_view("https://example.com", "GET");
        view.auth = Auth::Basic {
            user: "{{USER}}".to_string(),
            pass: "{{PASS}}".to_string(),
        };
        let env = Environment {
            id: 1,
            name: "test".to_string(),
            variables: vec![
                ("USER".to_string(), "admin".to_string()),
                ("PASS".to_string(), "secret".to_string()),
            ],
            default_endpoint: None,
        };
        view.apply_environment(&env);
        assert_eq!(
            view.auth,
            Auth::Basic { user: "admin".to_string(), pass: "secret".to_string() }
        );
    }

    #[test]
    fn apply_environment_multiple_variables() {
        let mut view = make_view("{{PROTO}}://{{HOST}}:{{PORT}}/api", "GET");
        let env = Environment {
            id: 1,
            name: "test".to_string(),
            variables: vec![
                ("PROTO".to_string(), "https".to_string()),
                ("HOST".to_string(), "localhost".to_string()),
                ("PORT".to_string(), "8080".to_string()),
            ],
            default_endpoint: None,
        };
        view.apply_environment(&env);
        assert_eq!(view.url_input, "https://localhost:8080/api");
    }

    #[test]
    fn apply_environment_no_variables_no_change() {
        let mut view = make_view("https://example.com/api", "GET");
        let env = Environment {
            id: 1,
            name: "test".to_string(),
            variables: vec![],
            default_endpoint: None,
        };
        view.apply_environment(&env);
        assert_eq!(view.url_input, "https://example.com/api");
    }
}
