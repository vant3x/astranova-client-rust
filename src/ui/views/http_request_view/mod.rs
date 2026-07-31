mod auth_tab;
mod body_tab;
mod builders;
mod cookies_tab;
mod helpers;
mod response_area;
mod scripts_tab;
mod settings_tab;
mod snippets_panel;
mod tests;
mod views;

use crate::data::auth::{Auth, AuthType};
use crate::data::auth_input::AuthInput;
use crate::http_client::config::RequestConfig;
use crate::http_client::response::HttpResponse;
use crate::http_client::snippets::SnippetFormat;
use crate::protocols::scripts::RequestScripts;
use crate::ui::components::key_value_editor::{self, KeyValueEditor};
use crate::ui::request_status::RequestStatus;
use iced::highlighter;
use iced::widget::image::Handle as ImageHandle;
use iced::widget::text_editor;
use serde::{Deserialize, Serialize};
use std::time::Duration;

pub(crate) const LOGO_BG_BYTES: &[u8] = include_bytes!("../../../../assets/astra-bg.png");

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct CookieSnapshot {
    pub name: String,
    pub value: String,
    pub domain: String,
    pub path: String,
    pub secure: bool,
    pub http_only: bool,
    pub same_site: String,
    pub expires: Option<String>,
}

pub(crate) static HTTP_METHODS: [&str; 7] =
    ["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS"];

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
    FormUrlencoded,
}

impl BodyType {
    pub const ALL: [BodyType; 3] = [
        BodyType::Text,
        BodyType::Multipart,
        BodyType::FormUrlencoded,
    ];
}

impl std::fmt::Display for BodyType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BodyType::Text => write!(f, "Text"),
            BodyType::Multipart => write!(f, "Multipart/Form-Data"),
            BodyType::FormUrlencoded => write!(f, "Form URL-Encoded"),
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
#[allow(dead_code)]
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
    ResponseReceived(Result<HttpResponse, crate::error::AppError>, Vec<String>),
    CopyResponse,
    CopyHeaders,
    CopyBody,
    CopyError(String),
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
    FormNameChanged(usize, String),
    FormValueChanged(usize, String),
    AddFormEntry,
    RemoveFormEntry(usize),
    RetryCountChanged(String),
    RetryBackoffChanged(String),
    ProxyUrlChanged(String),
    ProxyAuthUsernameChanged(String),
    ProxyAuthPasswordChanged(String),
    VerifySslToggled(bool),
    CookieStoreToggled(bool),
    CaCertPathChanged(String),
    ClientCertPathChanged(String),
    ClientKeyPathChanged(String),
    ThemeSelected(highlighter::Theme),
    ShowSnippets,
    HideSnippets,
    SnippetFormatSelected(SnippetFormat),
    CopySnippet,
    ImportCurlToggle,
    ImportCurlChanged(String),
    ImportCurlSubmit,
    ResetSettings,
    ToggleWordWrap,
    OAuth2StartAuth,
    OAuth2RefreshToken,
    OAuth2StartDeviceAuth,
    OAuth2CopyUserCode(String),
    OAuth2CopyAccessToken(String),
    OAuth2CopyRefreshToken(String),
    OAuth2AutoPollToggle(bool),
    CurlImported,
    ToggleResponseSearch,
    ResponseSearchChanged(String),
    SearchNext,
    SearchPrev,
    DownloadResponse,
    ResponseFileSaved(Result<String, String>),
    ToggleImagePreview,
    CancelRequest,
    StreamEvent(usize, crate::http_client::response::HttpStreamEvent),
    ToggleBearerTokenVisible,
    ToggleApiKeyValueVisible,
    SetIdle,
    ClearKeychainSecrets,
    ClearCookies,
    CookieManagerMsg(crate::ui::views::cookie_manager::Message),
    ScriptTabSelected(ScriptTab),
    PreRequestScriptChanged(text_editor::Action),
    PostResponseScriptChanged(text_editor::Action),
    SaveScripts,
    ScriptsSaved(Result<(), String>),
    CopyScripts,
    PasteScripts,
    ScriptOutputUpdated(crate::ui::views::http_request_view::ScriptOutput),
    SessionNewNameChanged(String),
    SessionSave(String),
    SessionLoad(String),
    SessionDelete(String),
    SessionConfirmDelete(String),
    SessionCancelDelete,
    SessionRenameStart(String),
    SessionRenameValueChanged(String),
    SessionRenameConfirm,
    SessionRenameCancel,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum TabId {
    #[default]
    Body,
    Headers,
    Params,
    Authorization,
    Cookies,
    Scripts,
    Settings,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ResponseTab {
    #[default]
    Body,
    Headers,
    Timeline,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ScriptTab {
    #[default]
    PreRequest,
    PostResponse,
    Output,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ScriptOutput {
    pub pre_logs: Vec<String>,
    pub pre_errors: Vec<String>,
    pub post_logs: Vec<String>,
    pub post_errors: Vec<String>,
    pub extracted_vars: Vec<(String, String)>,
    #[serde(default)]
    pub test_results: Vec<TestResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResult {
    pub name: String,
    pub passed: bool,
    pub message: Option<String>,
}

pub use crate::ui::theme::method_color;

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ResponseState {
    pub last_response: Option<HttpResponse>,
    pub response_body_editor: text_editor::Content,
    pub status_code: Option<u16>,
    pub content_type: Option<String>,
    pub response_duration: Option<Duration>,
    pub response_size: Option<u64>,
    pub streaming_body: String,
    pub streaming_chunks_count: u32,
    pub show_image_preview: bool,
    pub image_preview_handle: Option<ImageHandle>,
}

impl Default for ResponseState {
    fn default() -> Self {
        Self {
            last_response: None,
            response_body_editor: text_editor::Content::new(),
            status_code: None,
            content_type: None,
            response_duration: None,
            response_size: None,
            streaming_body: String::new(),
            streaming_chunks_count: 0,
            show_image_preview: false,
            image_preview_handle: None,
        }
    }
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ScriptState {
    pub scripts: RequestScripts,
    pub pre_request_script_editor: text_editor::Content,
    pub post_response_script_editor: text_editor::Content,
    pub active_script_tab: ScriptTab,
    pub script_output: ScriptOutput,
}

impl Default for ScriptState {
    fn default() -> Self {
        Self {
            scripts: RequestScripts::default(),
            pre_request_script_editor: text_editor::Content::new(),
            post_response_script_editor: text_editor::Content::new(),
            active_script_tab: ScriptTab::default(),
            script_output: ScriptOutput::default(),
        }
    }
}

#[derive(Debug, Clone, Default)]
#[allow(dead_code)]
pub struct SearchState {
    pub show_response_search: bool,
    pub response_search_query: String,
    pub response_search_matches: Vec<(usize, usize)>,
    pub response_search_index: usize,
    pub last_search_instant: Option<std::time::Instant>,
}

#[derive(Debug, Clone, Default)]
#[allow(dead_code)]
pub struct CookieTabState {
    pub cookie_count: usize,
    pub cookie_domain_count: usize,
    pub cookie_manager: crate::ui::views::cookie_manager::CookieManagerView,
    pub cookie_domains: Vec<(String, usize)>,
    pub cookie_domain_cookies: Vec<CookieSnapshot>,
}

#[derive(Debug, Clone, Default)]
#[allow(dead_code)]
pub struct SessionState {
    pub sessions: Vec<crate::persistence::database::Session>,
    pub new_session_name: String,
    pub selected_session: Option<String>,
    pub pending_delete_session: Option<String>,
    pub renaming_session: Option<String>,
    pub rename_value: String,
}

#[derive(Debug)]
pub struct HttpRequestView {
    pub url_input: String,
    pub method: String,
    pub body_input: text_editor::Content,
    pub auth: Auth,
    pub headers_editor: KeyValueEditor,
    pub params_editor: KeyValueEditor,
    pub(crate) active_tab: TabId,
    pub(crate) active_response_tab: ResponseTab,
    pub(crate) request_status: RequestStatus,
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
    pub(crate) multipart_next_id: usize,
    pub form_entries: Vec<MultipartEntry>,
    pub(crate) form_next_id: usize,
    pub highlighter_theme: highlighter::Theme,
    pub show_snippets: bool,
    pub show_import_curl: bool,
    pub import_curl_input: String,
    pub snippet_format: SnippetFormat,
    pub snippet_content: text_editor::Content,
    pub word_wrap: bool,
    pub pending_request_data: Option<String>,
    pub(crate) logo_handle: ImageHandle,
    pub streaming_body: String,
    pub streaming_chunks_count: u32,
    pub show_response_search: bool,
    pub response_search_query: String,
    pub response_search_matches: Vec<(usize, usize)>,
    pub response_search_index: usize,
    pub show_image_preview: bool,
    pub image_preview_handle: Option<ImageHandle>,
    pub show_bearer_token: bool,
    pub show_api_key_value: bool,
    pub abort_handle: Option<iced::task::Handle>,
    pub scripts: RequestScripts,
    pub pre_request_script_editor: text_editor::Content,
    pub post_response_script_editor: text_editor::Content,
    pub active_script_tab: ScriptTab,
    pub script_output: ScriptOutput,
    pub cookie_count: usize,
    pub highlight_content: Option<text_editor::Content>,
    pub cookie_domain_count: usize,
    pub cookie_manager: crate::ui::views::cookie_manager::CookieManagerView,
    pub cookie_domains: Vec<(String, usize)>,
    pub cookie_domain_cookies: Vec<CookieSnapshot>,
    pub sessions: Vec<crate::persistence::database::Session>,
    pub new_session_name: String,
    pub selected_session: Option<String>,
    pub pending_delete_session: Option<String>,
    pub renaming_session: Option<String>,
    pub rename_value: String,
    pub last_search_instant: Option<std::time::Instant>,
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
            form_entries: self.form_entries.clone(),
            form_next_id: self.form_next_id,
            highlighter_theme: self.highlighter_theme,
            show_snippets: self.show_snippets,
            show_import_curl: self.show_import_curl,
            import_curl_input: self.import_curl_input.clone(),
            snippet_format: self.snippet_format,
            snippet_content: text_editor::Content::with_text(&self.snippet_content.text()),
            word_wrap: self.word_wrap,
            pending_request_data: self.pending_request_data.clone(),
            logo_handle: self.logo_handle.clone(),
            streaming_body: String::new(),
            streaming_chunks_count: 0,
            show_response_search: self.show_response_search,
            response_search_query: self.response_search_query.clone(),
            response_search_matches: self.response_search_matches.clone(),
            response_search_index: self.response_search_index,
            show_image_preview: self.show_image_preview,
            image_preview_handle: self.image_preview_handle.clone(),
            show_bearer_token: self.show_bearer_token,
            show_api_key_value: self.show_api_key_value,
            abort_handle: None,
            scripts: self.scripts.clone(),
            pre_request_script_editor: text_editor::Content::with_text(
                &self.pre_request_script_editor.text(),
            ),
            post_response_script_editor: text_editor::Content::with_text(
                &self.post_response_script_editor.text(),
            ),
            active_script_tab: self.active_script_tab.clone(),
            script_output: self.script_output.clone(),
            cookie_count: self.cookie_count,
            highlight_content: self.highlight_content.as_ref().map(|c| {
                text_editor::Content::with_text(&c.text())
            }),
            cookie_domain_count: self.cookie_domain_count,
            cookie_manager: self.cookie_manager.clone(),
            cookie_domains: self.cookie_domains.clone(),
            cookie_domain_cookies: self.cookie_domain_cookies.clone(),
            sessions: self.sessions.clone(),
            new_session_name: self.new_session_name.clone(),
            selected_session: self.selected_session.clone(),
            pending_delete_session: self.pending_delete_session.clone(),
            renaming_session: self.renaming_session.clone(),
            rename_value: self.rename_value.clone(),
            last_search_instant: None,
        }
    }
}

impl HttpRequestView {
    pub fn clone_for_send(&self) -> Self {
        Self {
            url_input: self.url_input.clone(),
            method: self.method.clone(),
            body_input: text_editor::Content::with_text(&self.body_input.text()),
            auth: self.auth.clone(),
            headers_editor: self.headers_editor.clone(),
            params_editor: self.params_editor.clone(),
            request_content_type: self.request_content_type,
            request_config: self.request_config.clone(),
            body_type: self.body_type,
            multipart_entries: self.multipart_entries.clone(),
            multipart_next_id: self.multipart_next_id,
            form_entries: self.form_entries.clone(),
            form_next_id: self.form_next_id,
            scripts: self.scripts.clone(),
            active_tab: self.active_tab.clone(),
            active_response_tab: self.active_response_tab.clone(),
            request_status: self.request_status.clone(),
            last_response: None,
            response_body_editor: text_editor::Content::new(),
            status_code: None,
            content_type: None,
            response_duration: None,
            response_size: None,
            highlighter_theme: self.highlighter_theme,
            show_snippets: false,
            show_import_curl: false,
            import_curl_input: String::new(),
            snippet_format: self.snippet_format,
            snippet_content: text_editor::Content::new(),
            word_wrap: self.word_wrap,
            pending_request_data: None,
            logo_handle: self.logo_handle.clone(),
            streaming_body: String::new(),
            streaming_chunks_count: 0,
            show_response_search: false,
            response_search_query: String::new(),
            response_search_matches: Vec::new(),
            response_search_index: 0,
            show_image_preview: false,
            image_preview_handle: None,
            show_bearer_token: self.show_bearer_token,
            show_api_key_value: self.show_api_key_value,
            abort_handle: None,
            pre_request_script_editor: text_editor::Content::with_text(
                &self.pre_request_script_editor.text(),
            ),
            post_response_script_editor: text_editor::Content::with_text(
                &self.post_response_script_editor.text(),
            ),
            active_script_tab: self.active_script_tab.clone(),
            script_output: ScriptOutput::default(),
            cookie_count: 0,
            highlight_content: None,
            cookie_domain_count: 0,
            cookie_manager: Default::default(),
            cookie_domains: Vec::new(),
            cookie_domain_cookies: Vec::new(),
            sessions: Vec::new(),
            new_session_name: String::new(),
            selected_session: None,
            pending_delete_session: None,
            renaming_session: None,
            rename_value: String::new(),
            last_search_instant: None,
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
            form_entries: vec![MultipartEntry {
                id: 0,
                name: String::new(),
                value: String::new(),
                is_file: false,
            }],
            form_next_id: 1,
            highlighter_theme: highlighter::Theme::SolarizedDark,
            show_snippets: false,
            show_import_curl: false,
            import_curl_input: String::new(),
            snippet_format: SnippetFormat::Curl,
            snippet_content: text_editor::Content::new(),
            word_wrap: false,
            pending_request_data: None,
            logo_handle: ImageHandle::from_bytes(bytes::Bytes::from_static(LOGO_BG_BYTES)),
            streaming_body: String::new(),
            streaming_chunks_count: 0,
            show_response_search: false,
            response_search_query: String::new(),
            response_search_matches: Vec::new(),
            response_search_index: 0,
            show_image_preview: false,
            image_preview_handle: None,
            show_bearer_token: false,
            show_api_key_value: false,
            abort_handle: None,
            scripts: RequestScripts::default(),
            pre_request_script_editor: text_editor::Content::new(),
            post_response_script_editor: text_editor::Content::new(),
            active_script_tab: ScriptTab::default(),
            script_output: ScriptOutput::default(),
            cookie_count: 0,
            highlight_content: None,
            cookie_domain_count: 0,
            cookie_manager: crate::ui::views::cookie_manager::CookieManagerView::default(),
            cookie_domains: Vec::new(),
            cookie_domain_cookies: Vec::new(),
            sessions: Vec::new(),
            new_session_name: String::new(),
            selected_session: None,
            pending_delete_session: None,
            renaming_session: None,
            rename_value: String::new(),
            last_search_instant: None,
        }
    }
}

impl HttpRequestView {
    pub fn is_body_empty(text: &str) -> bool {
        let trimmed = text.trim();
        trimmed.is_empty() || trimmed == "\n"
    }

    pub fn update(&mut self, message: Message) {
        match message {
            Message::UrlInputChanged(url) => {
                if url.trim_start().starts_with("curl ") {
                    if let Ok(parsed) = crate::import::curl::parse_curl(&url) {
                        self.url_input = parsed.url;
                        self.method = parsed.method;
                        if let Some(body) = parsed.body {
                            self.body_input = text_editor::Content::with_text(&body);
                        }
                        self.headers_editor.entries.clear();
                        for (key, value) in parsed.headers {
                            self.headers_editor.entries.push(
                                crate::ui::components::key_value_editor::KeyValueEntry {
                                    id: self.headers_editor.entries.len(),
                                    key,
                                    value,
                                    secret: false,
                                },
                            );
                        }
                        if let (Some(user), Some(pass)) = (parsed.auth_user, parsed.auth_pass) {
                            self.auth = Auth::Basic { user, pass };
                        }
                        if parsed.insecure {
                            self.request_config.tls.verify_ssl = false;
                        }
                        if !parsed.form_fields.is_empty() {
                            self.body_type = BodyType::Multipart;
                            self.multipart_entries.clear();
                            for (i, (name, value)) in parsed.form_fields.into_iter().enumerate() {
                                let is_file = value.starts_with('@');
                                let file_value = if is_file {
                                    value[1..].to_string()
                                } else {
                                    value
                                };
                                self.multipart_entries.push(MultipartEntry {
                                    id: i,
                                    name,
                                    value: file_value,
                                    is_file,
                                });
                            }
                            self.multipart_next_id = self.multipart_entries.len();
                        }
                    } else {
                        self.url_input = url;
                    }
                } else {
                    // Parse query params from URL and sync to params editor
                    if let Ok(parsed_url) = reqwest::Url::parse(&url) {
                        let params: Vec<(String, String)> = parsed_url
                            .query_pairs()
                            .map(|(k, v)| (k.to_string(), v.to_string()))
                            .collect();
                        if !params.is_empty() {
                            self.params_editor.entries = params
                                .into_iter()
                                .enumerate()
                                .map(|(i, (k, v))| {
                                    crate::ui::components::key_value_editor::KeyValueEntry {
                                        id: i,
                                        key: k,
                                        value: v,
                                        secret: false,
                                    }
                                })
                                .collect();
                        }
                    }
                    self.url_input = url;
                }
            }
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
                    AuthType::OAuth2 => Auth::OAuth2(Box::default()),
                };
            }
            Message::AuthInputChanged(input) => {
                self.auth.apply_input(input);
            }
            Message::HeadersEditor(msg) => self.headers_editor.update(msg),
            Message::ParamsEditor(msg) => self.params_editor.update(msg),
            Message::BodyInputChanged(action) => self.body_input.perform(action),
            Message::RequestContentTypeSelected(content_type) => {
                self.request_content_type = content_type
            }
            Message::SendRequest => {}
            Message::SetLoading => {
                self.request_status = RequestStatus::Loading {
                    started_at: std::time::Instant::now(),
                };
                self.last_response = None;
                self.response_body_editor = text_editor::Content::new();
                self.highlight_content = None;
                self.streaming_body.clear();
                self.streaming_chunks_count = 0;
                self.status_code = None;
                self.content_type = None;
                self.response_duration = None;
                self.response_size = None;
                self.show_image_preview = false;
                self.image_preview_handle = None;
            }
            Message::SetIdle => {
                self.request_status = RequestStatus::Idle;
            }
            Message::StreamEvent(_tab_index, event) => {
                use crate::http_client::response::HttpStreamEvent;
                match event {
                    HttpStreamEvent::HeadersReceived { status, headers, url, method: _ } => {
                        self.status_code = Some(status);
                        let content_type = headers
                            .iter()
                            .find(|(k, _)| k.eq_ignore_ascii_case("content-type"))
                            .map(|(_, v)| v.clone())
                            .unwrap_or_else(|| "unknown".to_string());
                        self.content_type = Some(content_type);
                        self.streaming_body.clear();
                        self.streaming_chunks_count = 0;
                        self.last_response = Some(crate::http_client::response::HttpResponse {
                            url,
                            method: self.method.parse().unwrap_or(crate::http_client::request::HttpMethod::Get),
                            status,
                            headers,
                            body: String::new(),
                            body_encoding: crate::http_client::response::BodyEncoding::Text,
                            duration: std::time::Duration::ZERO,
                            size: 0,
                            redirect_chain: Vec::new(),
                        });
                        // Don't reset started_at here - preserve the original request start time
                    }
                    HttpStreamEvent::BodyChunk(chunk) => {
                        if let Ok(text) = String::from_utf8(chunk) {
                            self.streaming_body.push_str(&text);
                            self.streaming_chunks_count += 1;
                            let should_update = self.streaming_chunks_count == 1
                                || self.streaming_chunks_count.is_multiple_of(50);
                            if should_update {
                                let preview = if self.streaming_body.len() > 500_000 {
                                    &self.streaming_body[self.streaming_body.len() - 500_000..]
                                } else {
                                    &self.streaming_body
                                };
                                self.response_body_editor = text_editor::Content::with_text(preview);
                            }
                            // Don't set Success here - wait for StreamComplete
                        }
                    }
                    HttpStreamEvent::BodyChunkBinary(chunk) => {
                        use base64::Engine;
                        let encoded = base64::engine::general_purpose::STANDARD.encode(&chunk);
                        self.streaming_body.push_str(&encoded);
                        self.streaming_chunks_count += 1;
                        let should_update = self.streaming_chunks_count == 1
                            || self.streaming_chunks_count.is_multiple_of(50);
                        if should_update {
                            let preview = if self.streaming_body.len() > 500_000 {
                                &self.streaming_body[self.streaming_body.len() - 500_000..]
                            } else {
                                &self.streaming_body
                            };
                            self.response_body_editor = text_editor::Content::with_text(preview);
                        }
                        // Don't set Success here - wait for StreamComplete
                    }
                    HttpStreamEvent::StreamComplete { total_size } => {
                        let duration = if let RequestStatus::Loading { started_at } = self.request_status {
                            started_at.elapsed()
                        } else {
                            std::time::Duration::ZERO
                        };
                        let final_body = std::mem::take(&mut self.streaming_body);
                        self.response_size = Some(total_size);
                        self.response_duration = Some(duration);
                        let display = if final_body.len() > 500_000 {
                            let truncated_display = format!(
                                "... (showing last 500KB of {} bytes total) ...\n\n{}",
                                total_size,
                                &final_body[final_body.len() - 500_000..]
                            );
                            if let Some(ref mut resp) = self.last_response {
                                resp.body = final_body;
                                resp.size = total_size;
                                resp.duration = duration;
                            }
                            truncated_display
                        } else {
                            if let Some(ref mut resp) = self.last_response {
                                resp.body = final_body.clone();
                                resp.size = total_size;
                                resp.duration = duration;
                            }
                            final_body
                        };
                        self.response_body_editor = text_editor::Content::with_text(&display);
                        let display_len = display.len();
                        self.highlight_content = if display_len > 500_000 {
                            Some(text_editor::Content::with_text(&display[..500_000]))
                        } else {
                            None
                        };
                        self.streaming_chunks_count = 0;
                        self.request_status = RequestStatus::Success;
                    }
                    HttpStreamEvent::StreamError(e) => {
                        self.request_status = RequestStatus::Error(format!("Error: {}", e));
                        self.last_response = None;
                        self.streaming_body.clear();
                        self.highlight_content = None;
                        self.status_code = None;
                        self.content_type = None;
                    }
                }
            }
            Message::ResponseReceived(result, _warnings) => match result {
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

                    let is_image = content_type.contains("image/");
                    let formatted_body = if content_type.contains("application/json")
                        && response.body.len() < 100_000
                    {
                        serde_json::from_str::<serde_json::Value>(&response.body)
                            .ok()
                            .and_then(|json_value| serde_json::to_string_pretty(&json_value).ok())
                            .unwrap_or_else(|| response.body.clone())
                    } else if is_image
                        && response.body_encoding
                            == crate::http_client::response::BodyEncoding::Base64
                    {
                        // Decode base64 image and create preview handle
                        if let Ok(bytes) = base64::Engine::decode(
                            &base64::engine::general_purpose::STANDARD,
                            &response.body,
                        ) {
                            self.image_preview_handle =
                                Some(iced::widget::image::Handle::from_bytes(bytes));
                            self.show_image_preview = true;
                            format!(
                                "[Image: {} bytes, base64 decoded for preview]",
                                response.body.len()
                            )
                        } else {
                            response.body.clone()
                        }
                    } else {
                        response.body.clone()
                    };

                    self.response_body_editor = text_editor::Content::with_text(&formatted_body);
                    let body_len = formatted_body.len();
                    self.highlight_content = if body_len > 500_000 {
                        Some(text_editor::Content::with_text(&formatted_body[..500_000]))
                    } else {
                        None
                    };
                    self.last_response = Some(response);
                    self.request_status = RequestStatus::Success;
                }
                Err(e) => {
                    self.request_status = RequestStatus::Error(format!("Error: {}", e));
                    self.last_response = None;
                    self.response_body_editor = text_editor::Content::new();
                    self.highlight_content = None;
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
            Message::CopyError(error_text) => {
                if let Ok(mut clipboard) = arboard::Clipboard::new() {
                    let _ = clipboard.set_text(error_text);
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
                if url.is_empty() {
                    self.request_config.proxy_url = None;
                } else {
                    self.request_config.proxy_url = Some(url);
                }
            }
            Message::ProxyAuthUsernameChanged(username) => {
                let url = self
                    .request_config
                    .proxy_url
                    .clone()
                    .or_else(|| self.request_config.proxy.as_ref().map(|p| p.url.clone()))
                    .unwrap_or_default();
                let password = self
                    .request_config
                    .proxy
                    .as_ref()
                    .and_then(|p| p.auth.as_ref())
                    .map(|a| a.password.clone())
                    .unwrap_or_default();
                self.request_config.proxy = Some(crate::http_client::config::ProxyConfig {
                    url,
                    auth: Some(crate::http_client::config::ProxyAuth { username, password }),
                });
            }
            Message::ProxyAuthPasswordChanged(password) => {
                let url = self
                    .request_config
                    .proxy_url
                    .clone()
                    .or_else(|| self.request_config.proxy.as_ref().map(|p| p.url.clone()))
                    .unwrap_or_default();
                let username = self
                    .request_config
                    .proxy
                    .as_ref()
                    .and_then(|p| p.auth.as_ref())
                    .map(|a| a.username.clone())
                    .unwrap_or_default();
                self.request_config.proxy = Some(crate::http_client::config::ProxyConfig {
                    url,
                    auth: Some(crate::http_client::config::ProxyAuth { username, password }),
                });
            }
            Message::VerifySslToggled(verify) => {
                self.request_config.tls.verify_ssl = verify;
            }
            Message::CookieStoreToggled(enabled) => {
                self.request_config.cookie_store = enabled;
            }
            Message::CaCertPathChanged(path) => {
                self.request_config.tls.ca_cert_path =
                    if path.is_empty() { None } else { Some(path) };
            }
            Message::ClientCertPathChanged(path) => {
                self.request_config.tls.client_cert_path =
                    if path.is_empty() { None } else { Some(path) };
            }
            Message::ClientKeyPathChanged(path) => {
                self.request_config.tls.client_key_path =
                    if path.is_empty() { None } else { Some(path) };
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
            Message::FormNameChanged(id, name) => {
                if let Some(entry) = self.form_entries.iter_mut().find(|e| e.id == id) {
                    entry.name = name;
                }
            }
            Message::FormValueChanged(id, value) => {
                if let Some(entry) = self.form_entries.iter_mut().find(|e| e.id == id) {
                    entry.value = value;
                }
            }
            Message::AddFormEntry => {
                self.form_entries.push(MultipartEntry {
                    id: self.form_next_id,
                    name: String::new(),
                    value: String::new(),
                    is_file: false,
                });
                self.form_next_id += 1;
            }
            Message::RemoveFormEntry(id) => {
                self.form_entries.retain(|e| e.id != id);
            }
            Message::ShowSnippets => {
                self.show_snippets = true;
                if let Ok(request) = self.build_request() {
                    let code =
                        crate::http_client::snippets::generate(&request, self.snippet_format);
                    self.snippet_content = text_editor::Content::with_text(&code);
                }
            }
            Message::HideSnippets => {
                self.show_snippets = false;
            }
            Message::SnippetFormatSelected(format) => {
                self.snippet_format = format;
                if let Ok(request) = self.build_request() {
                    let code =
                        crate::http_client::snippets::generate(&request, self.snippet_format);
                    self.snippet_content = text_editor::Content::with_text(&code);
                }
            }
            Message::CopySnippet => {
                let text = self.snippet_content.text();
                if let Ok(mut clipboard) = arboard::Clipboard::new() {
                    let _ = clipboard.set_text(text);
                }
            }
            Message::ImportCurlToggle => {
                self.show_import_curl = !self.show_import_curl;
                if self.show_import_curl {
                    self.import_curl_input.clear();
                    self.show_snippets = true;
                }
            }
            Message::ImportCurlChanged(input) => {
                self.import_curl_input = input;
            }
            Message::ImportCurlSubmit => {
                let curl_input = self.import_curl_input.clone();
                match crate::import::curl::parse_curl(&curl_input) {
                    Ok(result) => {
                        self.url_input = result.url;
                        self.method = result.method;
                        self.headers_editor = KeyValueEditor::new("Add Header".to_string());
                        for (key, value) in result.headers {
                            self.headers_editor
                                .update(crate::ui::components::key_value_editor::Message::AddEntry);
                            let entry_id = self
                                .headers_editor
                                .entries
                                .last()
                                .map(|e| e.id)
                                .unwrap_or(0);
                            self.headers_editor.update(
                                crate::ui::components::key_value_editor::Message::EntryKeyChanged(
                                    entry_id, key,
                                ),
                            );
                            self.headers_editor.update(
                                crate::ui::components::key_value_editor::Message::EntryValueChanged(
                                    entry_id, value,
                                ),
                            );
                        }
                        if let Some(body) = result.body {
                            self.body_input = text_editor::Content::with_text(&body);
                        }
                        self.show_import_curl = false;
                        self.import_curl_input.clear();
                    }
                    Err(e) => {
                        log::error!("Failed to parse cURL: {}", e);
                    }
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
            Message::OAuth2CopyUserCode(code) => {
                if let Ok(mut clipboard) = arboard::Clipboard::new() {
                    let _ = clipboard.set_text(code);
                }
            }
            Message::OAuth2CopyAccessToken(token) => {
                if let Ok(mut clipboard) = arboard::Clipboard::new() {
                    let _ = clipboard.set_text(token);
                }
            }
            Message::OAuth2CopyRefreshToken(token) => {
                if let Ok(mut clipboard) = arboard::Clipboard::new() {
                    let _ = clipboard.set_text(token);
                }
            }
            Message::OAuth2AutoPollToggle(_) => {
                // Handled in app.rs
            }
            Message::CurlImported => {
                // Handled in app.rs to show toast
            }
            Message::ToggleResponseSearch => {
                self.show_response_search = !self.show_response_search;
                if !self.show_response_search {
                    self.response_search_query.clear();
                    self.response_search_matches.clear();
                    self.response_search_index = 0;
                } else {
                    self.last_search_instant = None;
                    self.update_search_matches();
                }
            }
            Message::ResponseSearchChanged(query) => {
                let query_len = query.len();
                self.response_search_query = query;
                let now = std::time::Instant::now();
                let should_search = self
                    .last_search_instant
                    .map(|t| t.elapsed() >= std::time::Duration::from_millis(150))
                    .unwrap_or(true)
                    || query_len < self.response_search_matches.len();
                if should_search {
                    self.update_search_matches();
                    self.last_search_instant = Some(now);
                }
            }
            Message::SearchNext => {
                if self.response_search_matches.is_empty() && !self.response_search_query.is_empty()
                {
                    self.update_search_matches();
                    self.last_search_instant = Some(std::time::Instant::now());
                }
                if !self.response_search_matches.is_empty() {
                    self.response_search_index =
                        (self.response_search_index + 1) % self.response_search_matches.len();
                }
            }
            Message::SearchPrev => {
                if self.response_search_matches.is_empty() && !self.response_search_query.is_empty()
                {
                    self.update_search_matches();
                    self.last_search_instant = Some(std::time::Instant::now());
                }
                if !self.response_search_matches.is_empty() {
                    self.response_search_index = if self.response_search_index == 0 {
                        self.response_search_matches.len() - 1
                    } else {
                        self.response_search_index - 1
                    };
                }
            }
            Message::DownloadResponse => {
                // Handled in app.rs to use async file dialog
            }
            Message::ResponseFileSaved(_result) => {
                // Toast is handled in app.rs
            }
            Message::ToggleImagePreview => {
                self.show_image_preview = !self.show_image_preview;
            }
            Message::CancelRequest => {
                // Handled in app.rs via handle_http_request_msg:
                // aborts the in-flight Task and resets status to Idle.
            }
            Message::ToggleBearerTokenVisible => {
                self.show_bearer_token = !self.show_bearer_token;
            }
            Message::ToggleApiKeyValueVisible => {
                self.show_api_key_value = !self.show_api_key_value;
            }
            Message::ClearKeychainSecrets => {
                // Handled at app level - no view state change needed.
            }
            Message::ClearCookies => {
                // Handled at app level - no view state change needed.
            }
            Message::CookieManagerMsg(msg) => {
                use crate::ui::views::cookie_manager::Message as CmMsg;
                match msg {
                    CmMsg::DomainSelected(domain) => {
                        self.cookie_manager.selected_domain = Some(domain);
                    }
                    CmMsg::CookieSearchChanged(q) => {
                        self.cookie_manager.search_query = q;
                    }
                    CmMsg::StartEdit(domain, name, path) => {
                        let value = self
                            .cookie_domain_cookies
                            .iter()
                            .find(|c| c.domain == domain && c.name == name && c.path == path)
                            .map(|c| c.value.clone())
                            .unwrap_or_default();
                        self.cookie_manager.editing_cookie = Some((domain, name, path));
                        self.cookie_manager.edit_value = value;
                    }
                    CmMsg::EditValueChanged(v) => {
                        self.cookie_manager.edit_value = v;
                    }
                    CmMsg::SaveEdit | CmMsg::CancelEdit => {
                        self.cookie_manager.editing_cookie = None;
                        self.cookie_manager.edit_value.clear();
                    }
                    CmMsg::DeleteCookie(..)
                    | CmMsg::ClearDomain(..)
                    | CmMsg::ClearAll
                    | CmMsg::ImportCookies
                    | CmMsg::ImportData(_)
                    | CmMsg::ExportCookies
                    | CmMsg::ExportComplete(_)
                    | CmMsg::DeselectDomain
                    | CmMsg::RequestDeleteCookie(..)
                    | CmMsg::ConfirmDeleteCookie(..)
                    | CmMsg::CancelDeleteCookie
                    | CmMsg::RequestClearAll
                    | CmMsg::ConfirmClearAll
                    | CmMsg::CancelClearAll
                    | CmMsg::Close => {}
                }
            }
            Message::ScriptTabSelected(tab) => {
                self.active_script_tab = tab;
            }
            Message::PreRequestScriptChanged(action) => {
                self.pre_request_script_editor.perform(action);
            }
            Message::PostResponseScriptChanged(action) => {
                self.post_response_script_editor.perform(action);
            }
            Message::SaveScripts => {
                // Handled in app.rs
            }
            Message::ScriptsSaved(_) => {
                // Handled in app.rs
            }
            Message::CopyScripts => {
                if let Ok(scripts) = self.parse_scripts_from_editors() {
                    if let Ok(json) = scripts.to_json() {
                        if let Ok(mut clipboard) = arboard::Clipboard::new() {
                            let _ = clipboard.set_text(json);
                        }
                    }
                }
            }
            Message::PasteScripts => {
                if let Ok(mut clipboard) = arboard::Clipboard::new() {
                    if let Ok(text) = clipboard.get_text() {
                        if let Ok(scripts) =
                            crate::protocols::scripts::RequestScripts::from_json(&text)
                        {
                            self.load_scripts(&scripts);
                        }
                    }
                }
            }
            Message::ScriptOutputUpdated(output) => {
                self.script_output = output;
            }
            Message::SessionNewNameChanged(name) => {
                self.new_session_name = name;
            }
            Message::SessionSave(name) => {
                let name = name.trim().to_string();
                if name.is_empty() {
                    return;
                }
                self.new_session_name.clear();
                // Actual DB save handled in app.rs
            }
            Message::SessionLoad(session_id) => {
                // Actual data loading (cookies, headers, auth) handled in app.rs
                self.selected_session = Some(session_id);
            }
            Message::SessionDelete(session_id) => {
                self.pending_delete_session = Some(session_id);
            }
            Message::SessionConfirmDelete(session_id) => {
                self.sessions.retain(|s| s.id != session_id);
                self.pending_delete_session = None;
                if self.selected_session.as_ref() == Some(&session_id) {
                    self.selected_session = None;
                }
                // Persistence handled in app.rs
            }
            Message::SessionCancelDelete => {
                self.pending_delete_session = None;
            }
            Message::SessionRenameStart(session_id) => {
                if let Some(session) = self.sessions.iter().find(|s| s.id == session_id) {
                    self.rename_value = session.name.clone();
                }
                self.renaming_session = Some(session_id);
            }
            Message::SessionRenameValueChanged(value) => {
                self.rename_value = value;
            }
            Message::SessionRenameConfirm => {
                if let Some(ref session_id) = self.renaming_session.clone() {
                    let new_name = self.rename_value.trim().to_string();
                    if !new_name.is_empty() {
                        if let Some(session) =
                            self.sessions.iter_mut().find(|s| &s.id == session_id)
                        {
                            session.name = new_name;
                        }
                    }
                }
                self.renaming_session = None;
                self.rename_value.clear();
                // Persistence handled in app.rs
            }
            Message::SessionRenameCancel => {
                self.renaming_session = None;
                self.rename_value.clear();
            }
        }
    }

    fn update_search_matches(&mut self) {
        self.response_search_matches.clear();
        self.response_search_index = 0;
        if self.response_search_query.is_empty() {
            return;
        }
        let body_text = self.response_body_editor.text();
        let query_lower = self.response_search_query.to_lowercase();
        let body_lower = body_text.to_lowercase();
        let mut start = 0;
        while let Some(pos) = body_lower[start..].find(&query_lower) {
            let absolute_pos = start + pos;
            let line = body_text[..absolute_pos].lines().count();
            let col = absolute_pos
                - body_text[..absolute_pos]
                    .rfind('\n')
                    .map(|p| p + 1)
                    .unwrap_or(0);
            self.response_search_matches.push((line, col));
            start = absolute_pos + 1;
        }
    }

    pub fn load_scripts(&mut self, scripts: &RequestScripts) {
        self.scripts = scripts.clone();
        let pre_text = if !scripts.js_pre_request.trim().is_empty() {
            scripts.js_pre_request.clone()
        } else {
            scripts.pre_request.to_json().unwrap_or_default()
        };
        let post_text = if !scripts.js_post_response.trim().is_empty() {
            scripts.js_post_response.clone()
        } else {
            scripts.post_response.to_json().unwrap_or_default()
        };
        self.pre_request_script_editor = text_editor::Content::with_text(&pre_text);
        self.post_response_script_editor = text_editor::Content::with_text(&post_text);
    }

    pub fn parse_scripts_from_editors(&self) -> Result<RequestScripts, crate::error::AppError> {
        let pre_text = self.pre_request_script_editor.text();
        let post_text = self.post_response_script_editor.text();
        let pre_json = crate::protocols::scripts::Script::from_json(&pre_text).unwrap_or_default();
        let post_json =
            crate::protocols::scripts::Script::from_json(&post_text).unwrap_or_default();
        Ok(RequestScripts {
            pre_request: pre_json,
            post_response: post_json,
            js_pre_request: pre_text,
            js_post_response: post_text,
        })
    }
}
