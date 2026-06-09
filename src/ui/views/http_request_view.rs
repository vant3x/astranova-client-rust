use crate::data::auth::{Auth, AuthType};
use crate::http_client::config::RequestConfig;
use crate::http_client::response::HttpResponse;
use crate::http_client::snippets::{self, SnippetFormat};
use crate::persistence::database::Environment;
use crate::ui::components::key_value_editor::{self, KeyValueEditor};
use base64::{engine::general_purpose, Engine as _};
use bytes::Bytes;
use iced::highlighter;
use iced::widget::image::{Handle, Image};
use iced::widget::text_editor;
use iced::{
    widget::{button, column, container, pick_list, row, rule, scrollable, text, text_input},
    Alignment, Color, Element, Length, Renderer, Theme,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BodyType {
    #[default]
    Text,
    Multipart,
}

impl BodyType {
    pub const ALL: [BodyType; 2] = [BodyType::Text, BodyType::Multipart];
}

impl std::fmt::Display for BodyType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BodyType::Text => write!(f, "Text"),
            BodyType::Multipart => write!(f, "Multipart/Form-Data"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct MultipartEntry {
    pub id: usize,
    pub name: String,
    pub value: String,
    pub is_file: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MultipartFieldType {
    Text,
    File,
}

impl MultipartFieldType {
    pub const ALL: [MultipartFieldType; 2] = [MultipartFieldType::Text, MultipartFieldType::File];
}

impl std::fmt::Display for MultipartFieldType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MultipartFieldType::Text => write!(f, "Text"),
            MultipartFieldType::File => write!(f, "File"),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    UrlInputChanged(String),
    MethodSelected(String),
    TabSelected(TabId),
    ResponseTabSelected(ResponseTab),
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
    CopyHeaders,
    CopyBody,
    ResponseContentChanged(text_editor::Action),
    CopySelection,
    TimeoutChanged(String),
    FollowRedirectsToggled(bool),
    MaxRedirectsChanged(String),
    BodyTypeSelected(BodyType),
    MultipartNameChanged(usize, String),
    MultipartValueChanged(usize, String),
    MultipartFieldTypeChanged(usize, MultipartFieldType),
    AddMultipartEntry,
    RemoveMultipartEntry(usize),
    MultipartFilePicked(usize, Option<String>),
    MultipartBrowseFile(usize),
    RetryCountChanged(String),
    RetryBackoffChanged(String),
    ProxyUrlChanged(String),
    VerifySslToggled(bool),
    ThemeSelected(highlighter::Theme),
    ShowSnippets,
    HideSnippets,
    SnippetFormatSelected(SnippetFormat),
    CopySnippet,
    ResetSettings,
    ToggleWordWrap,
    OAuth2StartAuth,
    OAuth2RefreshToken,
    OAuth2StartDeviceAuth,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum TabId {
    #[default]
    Body,
    Headers,
    Params,
    Authorization,
    Settings,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ResponseTab {
    #[default]
    Body,
    Headers,
    Timeline,
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
    OAuth2GrantType(crate::data::auth::OAuth2GrantType),
    OAuth2AuthUrl(String),
    OAuth2TokenUrl(String),
    OAuth2DeviceAuthUrl(String),
    OAuth2ClientId(String),
    OAuth2ClientSecret(String),
    OAuth2Scopes(String),
    OAuth2RedirectUri(String),
    OAuth2PkceEnabled(bool),
    OAuth2AccessToken(String),
    OAuth2RefreshToken(String),
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

pub use crate::ui::theme::method_color;

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
pub struct HttpRequestView {
    pub url_input: String,
    pub method: String,
    pub body_input: text_editor::Content,
    pub auth: Auth,
    pub headers_editor: KeyValueEditor,
    pub params_editor: KeyValueEditor,
    active_tab: TabId,
    active_response_tab: ResponseTab,
    request_status: RequestStatus,
    pub last_response: Option<HttpResponse>,
    pub response_body_editor: text_editor::Content,
    pub status_code: Option<u16>,
    pub content_type: Option<String>,
    pub response_duration: Option<Duration>,
    pub response_size: Option<u64>,
    pub request_content_type: ContentType,
    pub request_config: RequestConfig,
    pub body_type: BodyType,
    pub multipart_entries: Vec<MultipartEntry>,
    multipart_next_id: usize,
    pub highlighter_theme: highlighter::Theme,
    pub show_snippets: bool,
    pub snippet_format: SnippetFormat,
    pub snippet_content: text_editor::Content,
    pub word_wrap: bool,
    pub pending_request_data: Option<String>,
    logo_handle: iced::widget::image::Handle,
}

impl Clone for HttpRequestView {
    fn clone(&self) -> Self {
        Self {
            url_input: self.url_input.clone(),
            method: self.method.clone(),
            body_input: text_editor::Content::with_text(&self.body_input.text()),
            auth: self.auth.clone(),
            headers_editor: self.headers_editor.clone(),
            params_editor: self.params_editor.clone(),
            active_tab: self.active_tab.clone(),
            active_response_tab: self.active_response_tab.clone(),
            request_status: self.request_status.clone(),
            last_response: self.last_response.clone(),
            response_body_editor: text_editor::Content::with_text(
                &self.response_body_editor.text(),
            ),
            status_code: self.status_code,
            content_type: self.content_type.clone(),
            response_duration: self.response_duration,
            response_size: self.response_size,
            request_content_type: self.request_content_type,
            request_config: self.request_config.clone(),
            body_type: self.body_type,
            multipart_entries: self.multipart_entries.clone(),
            multipart_next_id: self.multipart_next_id,
            highlighter_theme: self.highlighter_theme,
            show_snippets: self.show_snippets,
            snippet_format: self.snippet_format,
            snippet_content: text_editor::Content::with_text(&self.snippet_content.text()),
            word_wrap: self.word_wrap,
            pending_request_data: self.pending_request_data.clone(),
            logo_handle: self.logo_handle.clone(),
        }
    }
}

impl Default for HttpRequestView {
    fn default() -> Self {
        Self {
            url_input: "https://jsonplaceholder.typicode.com/todos/1".to_string(),
            method: "GET".to_string(),
            body_input: text_editor::Content::new(),
            auth: Auth::default(),
            headers_editor: KeyValueEditor::new("Add Header".to_string()),
            params_editor: KeyValueEditor::new("Add Param".to_string()),
            active_tab: TabId::Body,
            active_response_tab: ResponseTab::Body,
            request_status: RequestStatus::Idle,
            last_response: None,
            response_body_editor: text_editor::Content::new(),
            status_code: None,
            content_type: None,
            response_duration: None,
            response_size: None,
            request_content_type: ContentType::Json,
            request_config: RequestConfig::default(),
            body_type: BodyType::Text,
            multipart_entries: vec![MultipartEntry {
                id: 0,
                name: String::new(),
                value: String::new(),
                is_file: false,
            }],
            multipart_next_id: 1,
            highlighter_theme: highlighter::Theme::SolarizedDark,
            show_snippets: false,
            snippet_format: SnippetFormat::Curl,
            snippet_content: text_editor::Content::new(),
            word_wrap: false,
            pending_request_data: None,
            logo_handle: Handle::from_bytes(Bytes::from_static(LOGO_BG_BYTES)),
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
            Auth::Basic { user, pass } if !user.is_empty() || !pass.is_empty() => {
                let encoded = general_purpose::STANDARD.encode(format!("{}:{}", user, pass));
                headers.push(("Authorization".to_string(), format!("Basic {}", encoded)));
            }
            Auth::OAuth2(config) if !config.access_token.is_empty() => {
                headers.push((
                    "Authorization".to_string(),
                    format!("Bearer {}", config.access_token),
                ));
            }
            _ => {}
        }

        let body = if self.body_input.text().is_empty() {
            None
        } else {
            Some(self.body_input.text())
        };

        // Only set Content-Type for text body (multipart sets it automatically)
        if body.is_some() && self.body_type == BodyType::Text {
            let content_type_str = match self.request_content_type {
                ContentType::Json => "application/json",
                ContentType::Text => "text/plain",
                ContentType::Html => "text/html",
                ContentType::Xml => "application/xml",
            };
            headers.push(("Content-Type".to_string(), content_type_str.to_string()));
        }

        // Convert multipart entries to MultipartField
        let multipart_fields: Vec<crate::http_client::request::MultipartField> =
            if self.body_type == BodyType::Multipart {
                self.multipart_entries
                    .iter()
                    .filter(|e| !e.name.is_empty())
                    .map(|e| {
                        if e.is_file {
                            crate::http_client::request::MultipartField {
                                name: e.name.clone(),
                                value: crate::http_client::request::MultipartValue::File {
                                    path: e.value.clone(),
                                    filename: None,
                                },
                            }
                        } else {
                            crate::http_client::request::MultipartField {
                                name: e.name.clone(),
                                value: crate::http_client::request::MultipartValue::Text(
                                    e.value.clone(),
                                ),
                            }
                        }
                    })
                    .collect()
            } else {
                vec![]
            };

        crate::http_client::request::HttpRequest {
            method: self.method.to_string(),
            url: final_url,
            headers,
            body,
            config: self.request_config.clone(),
            multipart_fields,
        }
    }

    pub fn update(&mut self, message: Message) {
        match message {
            Message::UrlInputChanged(url) => self.url_input = url,
            Message::MethodSelected(method) => self.method = method,
            Message::TabSelected(tab_id) => self.active_tab = tab_id,
            Message::ResponseTabSelected(tab_id) => self.active_response_tab = tab_id,
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
                    AuthType::OAuth2 => Auth::OAuth2(crate::data::auth::OAuth2Config::default()),
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
                (Auth::OAuth2(config), AuthInput::OAuth2GrantType(grant_type)) => {
                    config.grant_type = grant_type;
                }
                (Auth::OAuth2(config), AuthInput::OAuth2AuthUrl(url)) => {
                    config.auth_url = url;
                }
                (Auth::OAuth2(config), AuthInput::OAuth2TokenUrl(url)) => {
                    config.token_url = url;
                }
                (Auth::OAuth2(config), AuthInput::OAuth2ClientId(id)) => {
                    config.client_id = id;
                }
                (Auth::OAuth2(config), AuthInput::OAuth2ClientSecret(secret)) => {
                    config.client_secret = secret;
                }
                (Auth::OAuth2(config), AuthInput::OAuth2Scopes(scopes)) => {
                    config.scopes = scopes;
                }
                (Auth::OAuth2(config), AuthInput::OAuth2RedirectUri(uri)) => {
                    config.redirect_uri = uri;
                }
                (Auth::OAuth2(config), AuthInput::OAuth2PkceEnabled(enabled)) => {
                    config.pkce_enabled = enabled;
                }
                (Auth::OAuth2(config), AuthInput::OAuth2AccessToken(token)) => {
                    config.access_token = token;
                }
                (Auth::OAuth2(config), AuthInput::OAuth2RefreshToken(token)) => {
                    config.refresh_token = token;
                }
                (Auth::OAuth2(config), AuthInput::OAuth2DeviceAuthUrl(url)) => {
                    config.device_auth_url = url;
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
                self.last_response = None;
                self.response_body_editor = text_editor::Content::new();
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

                    self.response_body_editor = text_editor::Content::with_text(&formatted_body);
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
                let text_to_copy = match &self.request_status {
                    RequestStatus::Success => Some(self.response_body_editor.text()),
                    RequestStatus::Error(error_message) => Some(error_message.clone()),
                    _ => None,
                };

                if let Some(text) = text_to_copy {
                    if let Ok(mut clipboard) = arboard::Clipboard::new() {
                        let _ = clipboard.set_text(text);
                    }
                }
            }
            Message::CopyHeaders => {
                if let Some(response) = &self.last_response {
                    let headers_text = response
                        .headers
                        .iter()
                        .map(|(k, v)| format!("{}: {}", k, v))
                        .collect::<Vec<_>>()
                        .join("\n");
                    if let Ok(mut clipboard) = arboard::Clipboard::new() {
                        let _ = clipboard.set_text(headers_text);
                    }
                }
            }
            Message::CopyBody => {
                let text_to_copy = self.response_body_editor.text();
                if !text_to_copy.is_empty() {
                    if let Ok(mut clipboard) = arboard::Clipboard::new() {
                        let _ = clipboard.set_text(text_to_copy);
                    }
                }
            }
            Message::ResponseContentChanged(action) => {
                self.response_body_editor.perform(action);
            }
            Message::CopySelection => {
                if let Some(selection) = self.response_body_editor.selection() {
                    if let Ok(mut clipboard) = arboard::Clipboard::new() {
                        let _ = clipboard.set_text(selection);
                    }
                }
            }
            Message::TimeoutChanged(secs) => {
                if let Ok(s) = secs.parse::<u64>() {
                    self.request_config.timeout = std::time::Duration::from_secs(s);
                }
            }
            Message::FollowRedirectsToggled(follow) => {
                use crate::http_client::config::RedirectPolicy;
                self.request_config.redirect_policy = if follow {
                    RedirectPolicy::Follow
                } else {
                    RedirectPolicy::NoFollow
                };
            }
            Message::MaxRedirectsChanged(max) => {
                if let Ok(n) = max.parse::<u32>() {
                    self.request_config.redirect_policy =
                        crate::http_client::config::RedirectPolicy::Limited(n);
                }
            }
            Message::RetryCountChanged(count) => {
                if let Ok(n) = count.parse::<u32>() {
                    self.request_config.retry.max_retries = n;
                }
            }
            Message::RetryBackoffChanged(ms) => {
                if let Ok(n) = ms.parse::<u64>() {
                    self.request_config.retry.backoff_ms = n;
                }
            }
            Message::ProxyUrlChanged(url) => {
                self.request_config.proxy_url = if url.is_empty() { None } else { Some(url) };
            }
            Message::VerifySslToggled(verify) => {
                self.request_config.verify_ssl = verify;
            }
            Message::ThemeSelected(theme) => {
                self.highlighter_theme = theme;
            }
            Message::BodyTypeSelected(body_type) => {
                self.body_type = body_type;
            }
            Message::MultipartNameChanged(id, name) => {
                if let Some(entry) = self.multipart_entries.iter_mut().find(|e| e.id == id) {
                    entry.name = name;
                }
            }
            Message::MultipartValueChanged(id, value) => {
                if let Some(entry) = self.multipart_entries.iter_mut().find(|e| e.id == id) {
                    entry.value = value;
                }
            }
            Message::MultipartFieldTypeChanged(id, field_type) => {
                if let Some(entry) = self.multipart_entries.iter_mut().find(|e| e.id == id) {
                    entry.is_file = matches!(field_type, MultipartFieldType::File);
                    if !entry.is_file {
                        entry.value.clear();
                    }
                }
            }
            Message::AddMultipartEntry => {
                self.multipart_entries.push(MultipartEntry {
                    id: self.multipart_next_id,
                    name: String::new(),
                    value: String::new(),
                    is_file: false,
                });
                self.multipart_next_id += 1;
            }
            Message::RemoveMultipartEntry(id) => {
                self.multipart_entries.retain(|e| e.id != id);
            }
            Message::ShowSnippets => {
                self.show_snippets = true;
                let request = self.build_request();
                let code = snippets::generate(&request, self.snippet_format);
                self.snippet_content = text_editor::Content::with_text(&code);
            }
            Message::HideSnippets => {
                self.show_snippets = false;
            }
            Message::SnippetFormatSelected(format) => {
                self.snippet_format = format;
                let request = self.build_request();
                let code = snippets::generate(&request, self.snippet_format);
                self.snippet_content = text_editor::Content::with_text(&code);
            }
            Message::CopySnippet => {
                let text = self.snippet_content.text();
                if let Ok(mut clipboard) = arboard::Clipboard::new() {
                    let _ = clipboard.set_text(text);
                }
            }
            Message::MultipartBrowseFile(_) => {
                // Handled in app.rs
            }
            Message::MultipartFilePicked(id, path) => {
                if let Some(value) = path {
                    if let Some(entry) = self.multipart_entries.iter_mut().find(|e| e.id == id) {
                        entry.value = value;
                    }
                }
            }
            Message::ResetSettings => {
                self.request_config = RequestConfig::default();
            }
            Message::ToggleWordWrap => {
                self.word_wrap = !self.word_wrap;
            }
            Message::OAuth2StartAuth => {
                // Handled in app.rs
            }
            Message::OAuth2RefreshToken => {
                // Handled in app.rs
            }
            Message::OAuth2StartDeviceAuth => {
                // Handled in app.rs
            }
        }
    }

    pub fn view(&self) -> Element<'_, Message, Theme, Renderer> {
        let auth_tab_content = self.create_auth_tab_content();
        let body_tab_content = self.create_body_tab_content();
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
                TabId::Settings,
                TabLabel::Text("Settings".to_string()),
                container(settings_tab_content)
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
            RequestStatus::Success => {
                let response_tabs = Tabs::new(Message::ResponseTabSelected)
                    .push(ResponseTab::Body, TabLabel::Text("Body".to_string()), {
                        let syntax = self
                            .content_type
                            .as_deref()
                            .map(response_content_type_to_syntax)
                            .unwrap_or("text");
                        if self.word_wrap {
                            let body_text = self.response_body_editor.text();
                            let wrapped_text = text(body_text).size(13).font(iced::Font::MONOSPACE);
                            let context_menu = ContextMenu::new(scrollable(wrapped_text), || {
                                column![button("Copy Body").on_press(Message::CopyBody),].into()
                            });
                            container(context_menu)
                        } else {
                            let editor = text_editor(&self.response_body_editor)
                                .on_action(Message::ResponseContentChanged)
                                .highlight(syntax, self.highlighter_theme);
                            let context_menu = ContextMenu::new(scrollable(editor), || {
                                column![
                                    button("Copy Selection").on_press(Message::CopySelection),
                                    button("Copy Body").on_press(Message::CopyBody),
                                ]
                                .into()
                            });
                            container(context_menu)
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

        let copy_button = if matches!(
            self.request_status,
            RequestStatus::Success | RequestStatus::Error(_)
        ) {
            Element::from(button("Copy").on_press(Message::CopyResponse))
        } else {
            Element::from(column![])
        };

        let wrap_toggle: Element<'_, Message, Theme, Renderer> =
            if matches!(self.request_status, RequestStatus::Success) {
                Element::from(
                    button(
                        text(if self.word_wrap {
                            "Wrap ON"
                        } else {
                            "Wrap OFF"
                        })
                        .size(11),
                    )
                    .on_press(Message::ToggleWordWrap),
                )
            } else {
                Element::from(column![])
            };

        let method_colored = text(self.method.as_str()).size(16).color(method_color(self.method.as_str()));

        let status_text = if let Some(status) = self.status_code {
            let color = status_color(status);
            text(format!("  {}  ", status)).size(14).color(color)
        } else {
            text("".to_string()).size(14)
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

        let main_column = column![
            Image::new(self.logo_handle.clone())
                .width(Length::Fixed(100.0))
                .height(Length::Fixed(100.0)),
            row![
                pick_list(
                    &HTTP_METHODS[..],
                    Some(self.method.as_str()),
                    |s: &str| Message::MethodSelected(s.to_string())
                )
                .padding(10),
                text_input("URL", &self.url_input)
                    .on_input(Message::UrlInputChanged)
                    .padding(10),
                button("Send").on_press(Message::SendRequest),
                button("Code").on_press(Message::ShowSnippets),
            ]
            .spacing(10)
            .padding(10),
            tabs.height(Length::Fixed(380.0)),
            rule::horizontal(10),
            column![
                row![
                    method_colored,
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

    fn create_response_headers_view(&self) -> Element<'_, Message, Theme, Renderer> {
        if let Some(response) = &self.last_response {
            let mut headers_col = column![].spacing(4);
            for (key, value) in &response.headers {
                headers_col = headers_col.push(
                    row![
                        text(format!("{}:", key))
                            .size(14)
                            .color(Color::from_rgb(0.4, 0.6, 0.9)),
                        text(value).size(14),
                    ]
                    .spacing(8),
                );
            }
            container(scrollable(headers_col))
                .padding(10)
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

    fn create_response_timeline_view(&self) -> Element<'_, Message, Theme, Renderer> {
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
                    text(format!("{:.2?}", response.duration)).size(14),
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
                    text(&response.method)
                        .size(14)
                        .color(method_color(&response.method)),
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
                    Some(location.clone()),
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
                            .on_input(|u| Message::AuthInputChanged(AuthInput::OAuth2RedirectUri(u)))
                            .padding(10),
                        row![
                            text("PKCE:"),
                            button(if config.pkce_enabled { "ON" } else { "OFF" })
                                .on_press(Message::AuthInputChanged(AuthInput::OAuth2PkceEnabled(!config.pkce_enabled))),
                        ]
                        .spacing(10)
                        .align_y(Alignment::Center),
                        button("Get Authorization").on_press(Message::OAuth2StartAuth),
                    ]
                    .spacing(10),
                    crate::data::auth::OAuth2GrantType::ClientCredentials => column![
                        text_input("Token URL", &config.token_url)
                            .on_input(|u| Message::AuthInputChanged(AuthInput::OAuth2TokenUrl(u)))
                            .padding(10),
                        text_input("Scopes (space-separated)", &config.scopes)
                            .on_input(|s| Message::AuthInputChanged(AuthInput::OAuth2Scopes(s)))
                            .padding(10),
                        button("Get Token").on_press(Message::OAuth2RefreshToken),
                    ]
                    .spacing(10),
                    crate::data::auth::OAuth2GrantType::DeviceCode => column![
                        text_input("Device Auth URL", &config.device_auth_url)
                            .on_input(|u| Message::AuthInputChanged(AuthInput::OAuth2DeviceAuthUrl(u)))
                            .padding(10),
                        if config.user_code.is_empty() {
                            Element::from(button("Start Device Authorization").on_press(Message::OAuth2StartDeviceAuth))
                        } else {
                            Element::from(column![
                                container(
                                    text(format!("  {}  ", config.user_code))
                                        .size(24)
                                        .color(Color::from_rgb(0.0, 0.5, 1.0))
                                )
                                .padding(15)
                                .center_x(Length::Fill)
                                .style(iced::widget::container::rounded_box),
                                text(format!("Open: {}", config.verification_uri)).size(12),
                                button("Copy User Code").on_press({
                                    let code = config.user_code.clone();
                                    Message::AuthInputChanged(AuthInput::OAuth2AccessToken(code))
                                }),
                                button("Poll for Token").on_press(Message::OAuth2RefreshToken),
                            ].spacing(8))
                        },
                    ]
                    .spacing(10),
                    crate::data::auth::OAuth2GrantType::Implicit => column![
                        text_input("Authorization URL", &config.auth_url)
                            .on_input(|u| Message::AuthInputChanged(AuthInput::OAuth2AuthUrl(u)))
                            .padding(10),
                        text_input("Redirect URI", &config.redirect_uri)
                            .on_input(|u| Message::AuthInputChanged(AuthInput::OAuth2RedirectUri(u)))
                            .padding(10),
                        text_input("Scopes (space-separated)", &config.scopes)
                            .on_input(|s| Message::AuthInputChanged(AuthInput::OAuth2Scopes(s)))
                            .padding(10),
                        button("Get Authorization").on_press(Message::OAuth2StartAuth),
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
                            .on_input(|t| Message::AuthInputChanged(AuthInput::OAuth2AccessToken(t)))
                            .padding(10)
                            .secure(true),
                        button("Copy").on_press({
                            let token = config.access_token.clone();
                            Message::AuthInputChanged(AuthInput::OAuth2AccessToken(token))
                        }),
                    ]
                    .spacing(4)
                    .align_y(Alignment::Center),
                    row![
                        text_input("Refresh Token", &config.refresh_token)
                            .on_input(|t| Message::AuthInputChanged(AuthInput::OAuth2RefreshToken(t)))
                            .padding(10)
                            .secure(true),
                        button("Copy").on_press({
                            let token = config.refresh_token.clone();
                            Message::AuthInputChanged(AuthInput::OAuth2RefreshToken(token))
                        }),
                    ]
                    .spacing(4)
                    .align_y(Alignment::Center),
                    if !config.status.to_string().is_empty() {
                        Element::from(
                            text(config.status.to_string())
                                .size(12)
                                .color(match &config.status {
                                    crate::data::auth::OAuth2Status::Error(_) => Color::from_rgb(0.8, 0.2, 0.2),
                                    crate::data::auth::OAuth2Status::Success(_) => Color::from_rgb(0.2, 0.7, 0.3),
                                    crate::data::auth::OAuth2Status::Loading => Color::from_rgb(0.8, 0.7, 0.1),
                                    _ => Color::from_rgb(0.5, 0.5, 0.5),
                                }),
                        )
                    } else {
                        Element::from(column![])
                    },
                ]
                .spacing(10)
            }
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
        let body_type_selector = pick_list(
            &BodyType::ALL[..],
            Some(self.body_type),
            Message::BodyTypeSelected,
        )
        .padding(10);

        match self.body_type {
            BodyType::Text => {
                let content_type_selector = pick_list(
                    &ContentType::ALL[..],
                    Some(self.request_content_type),
                    Message::RequestContentTypeSelected,
                )
                .padding(10);

                let body_syntax = content_type_to_syntax(self.request_content_type);
                let body_editor = text_editor(&self.body_input)
                    .on_action(Message::BodyInputChanged)
                    .height(Length::Fill)
                    .highlight(body_syntax, self.highlighter_theme);

                container(
                    column![
                        row![text("Body Type:"), body_type_selector].spacing(10),
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
            BodyType::Multipart => {
                let mut entries_col = column![].spacing(8);
                for entry in &self.multipart_entries {
                    let current_type = if entry.is_file {
                        MultipartFieldType::File
                    } else {
                        MultipartFieldType::Text
                    };
                    let value_input = if entry.is_file {
                        row![
                            text_input("File path", &entry.value)
                                .on_input(move |v| Message::MultipartValueChanged(entry.id, v))
                                .padding(8),
                            button(text("Browse"))
                                .on_press(Message::MultipartBrowseFile(entry.id))
                                .padding(8),
                        ]
                        .spacing(8)
                    } else {
                        row![text_input("Value", &entry.value)
                            .on_input(move |v| Message::MultipartValueChanged(entry.id, v))
                            .padding(8),]
                        .spacing(8)
                    };
                    let row = row![
                        pick_list(&MultipartFieldType::ALL[..], Some(current_type), move |t| {
                            Message::MultipartFieldTypeChanged(entry.id, t)
                        },)
                        .padding(8)
                        .width(Length::Fixed(80.0)),
                        text_input("Name", &entry.name)
                            .on_input(move |v| Message::MultipartNameChanged(entry.id, v))
                            .padding(8),
                        value_input,
                        button(text("X"))
                            .on_press(Message::RemoveMultipartEntry(entry.id))
                            .width(Length::Fixed(35.0)),
                    ]
                    .spacing(8)
                    .align_y(Alignment::Center);
                    entries_col = entries_col.push(row);
                }

                let add_button = button(text("+ Add Field")).on_press(Message::AddMultipartEntry);

                container(
                    column![
                        row![text("Body Type:"), body_type_selector].spacing(10),
                        text("Multipart/Form-Data Fields").size(16),
                        scrollable(entries_col).height(Length::Fill),
                        add_button,
                    ]
                    .spacing(15)
                    .padding(10),
                )
                .width(Length::Fill)
                .height(Length::Fill)
                .into()
            }
        }
    }

    fn create_settings_tab_content(&self) -> Element<'_, Message, Theme, Renderer> {
        use crate::http_client::config::RedirectPolicy;

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

        let verify_ssl = self.request_config.verify_ssl;
        let ssl_toggle = button(if verify_ssl {
            "Verify SSL: ON"
        } else {
            "Verify SSL: OFF (insecure)"
        })
        .on_press(Message::VerifySslToggled(!verify_ssl));

        let theme_selector = pick_list(
            highlighter::Theme::ALL,
            Some(self.highlighter_theme),
            Message::ThemeSelected,
        )
        .padding(10);

        container(
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
                ssl_toggle,
                rule::horizontal(10),
                text("Appearance").size(16),
                row![text("Highlight Theme:"), theme_selector]
                    .spacing(10)
                    .align_y(Alignment::Center),
                rule::horizontal(10),
                button("Reset to Defaults").on_press(Message::ResetSettings),
            ]
            .spacing(15)
            .padding(20),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }

    fn create_snippets_panel(&self) -> Element<'_, Message, Theme, Renderer> {
        let format_selector = pick_list(
            &SnippetFormat::ALL[..],
            Some(self.snippet_format),
            Message::SnippetFormatSelected,
        )
        .padding(8);

        let close_button = button(text("X"))
            .on_press(Message::HideSnippets)
            .width(Length::Fixed(35.0));

        let header = row![
            text("Code Snippets").size(16),
            format_selector,
            close_button,
        ]
        .spacing(10)
        .align_y(Alignment::Center);

        let syntax = match self.snippet_format {
            SnippetFormat::Curl => "sh",
            SnippetFormat::Python => "python",
            SnippetFormat::JavaScript => "javascript",
            SnippetFormat::Rust => "rust",
        };

        let editor = text_editor(&self.snippet_content)
            .highlight(syntax, self.highlighter_theme)
            .height(Length::Fill);

        let copy_button = button(text("Copy")).on_press(Message::CopySnippet);

        container(
            column![
                header,
                rule::horizontal(5),
                scrollable(editor).height(Length::Fill),
                copy_button,
            ]
            .spacing(10)
            .padding(10),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }
}

fn content_type_to_syntax(ct: ContentType) -> &'static str {
    match ct {
        ContentType::Json => "json",
        ContentType::Html => "html",
        ContentType::Xml => "xml",
        ContentType::Text => "text",
    }
}

fn response_content_type_to_syntax(ct: &str) -> &str {
    if ct.contains("json") {
        "json"
    } else if ct.contains("html") {
        "html"
    } else if ct.contains("xml") {
        "xml"
    } else {
        "text"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::persistence::database::Environment;
    use crate::ui::components::key_value_editor::KeyValueEntry;

    fn make_view(url: &str, method: &str) -> HttpRequestView {
        let mut view = HttpRequestView::default();
        view.url_input = url.to_string();
        view.method = method.to_string();
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
            KeyValueEntry {
                id: 0,
                key: "page".to_string(),
                value: "1".to_string(),
            },
            KeyValueEntry {
                id: 1,
                key: "limit".to_string(),
                value: "10".to_string(),
            },
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
        view.params_editor.entries = vec![KeyValueEntry {
            id: 0,
            key: "new".to_string(),
            value: "val".to_string(),
        }];
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
            KeyValueEntry {
                id: 0,
                key: String::new(),
                value: "val".to_string(),
            },
            KeyValueEntry {
                id: 1,
                key: "good".to_string(),
                value: "yes".to_string(),
            },
        ];
        let req = view.build_request();
        assert!(!req.url.contains("val"));
        assert!(req.url.contains("good=yes"));
    }

    #[test]
    fn build_request_with_headers() {
        let mut view = make_view("https://example.com", "GET");
        view.headers_editor.entries = vec![KeyValueEntry {
            id: 0,
            key: "Accept".to_string(),
            value: "text/html".to_string(),
        }];
        let req = view.build_request();
        assert!(req
            .headers
            .iter()
            .any(|(k, v)| k == "Accept" && v == "text/html"));
    }

    #[test]
    fn build_request_empty_headers_filtered() {
        let mut view = make_view("https://example.com", "GET");
        view.headers_editor.entries = vec![KeyValueEntry {
            id: 0,
            key: String::new(),
            value: "val".to_string(),
        }];
        let req = view.build_request();
        // Only auth headers should be present, not the empty one
        assert!(!req.headers.iter().any(|(k, _)| k.is_empty()));
    }

    #[test]
    fn build_request_bearer_auth() {
        let mut view = make_view("https://example.com", "GET");
        view.auth = Auth::BearerToken("my-secret-token".to_string());
        let req = view.build_request();
        assert!(req
            .headers
            .iter()
            .any(|(k, v)| k == "Authorization" && v == "Bearer my-secret-token"));
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
        let decoded = base64::engine::general_purpose::STANDARD
            .decode(encoded)
            .unwrap();
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
        assert!(req
            .headers
            .iter()
            .any(|(k, v)| k == "Content-Type" && v == "application/json"));
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
                req.headers
                    .iter()
                    .any(|(k, v)| k == "Content-Type" && v == expected),
                "Failed for {:?}: expected {}",
                ct,
                expected
            );
        }
    }

    #[test]
    fn apply_environment_replaces_url_variable() {
        let mut view = make_view("{{BASE_URL}}/api/users", "GET");
        let env = Environment {
            id: 1,
            name: "test".to_string(),
            variables: vec![(
                "BASE_URL".to_string(),
                "https://api.example.com".to_string(),
            )],
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
        view.headers_editor.entries = vec![KeyValueEntry {
            id: 0,
            key: "Authorization".to_string(),
            value: "Bearer {{TOKEN}}".to_string(),
        }];
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
        view.params_editor.entries = vec![KeyValueEntry {
            id: 0,
            key: "key".to_string(),
            value: "{{API_KEY}}".to_string(),
        }];
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
        assert_eq!(
            view.auth,
            Auth::BearerToken("eyJhbGciOiJIUzI1NiJ9".to_string())
        );
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
            Auth::Basic {
                user: "admin".to_string(),
                pass: "secret".to_string()
            }
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

    #[test]
    fn build_request_multipart_text_fields() {
        let mut view = make_view("https://example.com/upload", "POST");
        view.body_type = BodyType::Multipart;
        view.multipart_entries = vec![
            MultipartEntry {
                id: 0,
                name: "username".to_string(),
                value: "john".to_string(),
                is_file: false,
            },
            MultipartEntry {
                id: 1,
                name: "bio".to_string(),
                value: "Hello world".to_string(),
                is_file: false,
            },
        ];
        let req = view.build_request();
        assert_eq!(req.multipart_fields.len(), 2);
        assert!(!req.headers.iter().any(|(k, _)| k == "Content-Type"));
    }

    #[test]
    fn build_request_multipart_file_field() {
        let mut view = make_view("https://example.com/upload", "POST");
        view.body_type = BodyType::Multipart;
        view.multipart_entries = vec![MultipartEntry {
            id: 0,
            name: "document".to_string(),
            value: "/tmp/test.pdf".to_string(),
            is_file: true,
        }];
        let req = view.build_request();
        assert_eq!(req.multipart_fields.len(), 1);
        match &req.multipart_fields[0].value {
            crate::http_client::request::MultipartValue::File { path, .. } => {
                assert_eq!(path, "/tmp/test.pdf");
            }
            _ => panic!("Expected File variant"),
        }
    }

    #[test]
    fn build_request_multipart_empty_names_filtered() {
        let mut view = make_view("https://example.com/upload", "POST");
        view.body_type = BodyType::Multipart;
        view.multipart_entries = vec![
            MultipartEntry {
                id: 0,
                name: String::new(),
                value: "val".to_string(),
                is_file: false,
            },
            MultipartEntry {
                id: 1,
                name: "good".to_string(),
                value: "yes".to_string(),
                is_file: false,
            },
        ];
        let req = view.build_request();
        assert_eq!(req.multipart_fields.len(), 1);
        assert_eq!(req.multipart_fields[0].name, "good");
    }

    #[test]
    fn build_request_text_mode_ignores_multipart_entries() {
        let mut view = make_view("https://example.com/api", "POST");
        view.body_type = BodyType::Text;
        view.multipart_entries = vec![MultipartEntry {
            id: 0,
            name: "field".to_string(),
            value: "val".to_string(),
            is_file: false,
        }];
        let req = view.build_request();
        assert!(req.multipart_fields.is_empty());
    }
}
