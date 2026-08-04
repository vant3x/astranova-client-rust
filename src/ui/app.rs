use crate::cookie::CookieJar;
use crate::persistence::database::{self, Environment};
use crate::protocols::websocket::{WsEvent, WsSender, WsStatus};
use crate::ui::toast::ToastManager;
use crate::ui::views::collection_view::{self, CollectionView};
use crate::ui::views::environment_manager::{self, EnvironmentManagerView};
use crate::ui::views::history_view::{self, HistoryView};
use crate::ui::views::websocket_view::{self, WebSocketView};
use iced::{
    widget::{button, column, container, pick_list, row, rule, stack, text},
    Alignment, Element, Length, Subscription, Task,
};
use iced_aw::{TabLabel, Tabs};
use iced_fonts::lucide;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;

type HttpStreamReceiver =
    Arc<Mutex<Option<mpsc::UnboundedReceiver<crate::http_client::response::HttpStreamEvent>>>>;

use super::views::graphql_view::{self, GraphQLView};
use super::views::http_request_view::CookieSnapshot;
use super::views::http_request_view::{self, HttpRequestView};

use iced::futures::stream::BoxStream;
use iced::futures::{self, StreamExt as _};
use iced_futures::subscription::{from_recipe, EventStream, Recipe};

struct WsRecipe {
    receiver: Arc<Mutex<Option<mpsc::UnboundedReceiver<WsEvent>>>>,
    connection_id: u64,
}

impl Recipe for WsRecipe {
    type Output = Message;

    fn hash(&self, state: &mut iced_futures::subscription::Hasher) {
        use std::hash::Hash;
        std::any::TypeId::of::<WsRecipe>().hash(state);
        self.connection_id.hash(state);
    }

    fn stream(self: Box<Self>, _input: EventStream) -> BoxStream<'static, Message> {
        let receiver_arc = self.receiver;
        futures::stream::unfold(receiver_arc, |arc| async move {
            // Take the receiver out of the Option temporarily
            let mut receiver = {
                let mut guard = arc.lock().ok()?;
                guard.take()?
            };
            // Await outside the lock so we don't hold MutexGuard across await
            let event = receiver.recv().await?;
            // Put the receiver back
            if let Ok(mut guard) = arc.lock() {
                *guard = Some(receiver);
            }
            Some((Message::WsEvent(event), arc))
        })
        .boxed()
    }
}

struct HttpStreamRecipe {
    receiver:
        Arc<Mutex<Option<mpsc::UnboundedReceiver<crate::http_client::response::HttpStreamEvent>>>>,
    tab_index: usize,
    stream_id: u64,
}

impl Recipe for HttpStreamRecipe {
    type Output = Message;

    fn hash(&self, state: &mut iced_futures::subscription::Hasher) {
        use std::hash::Hash;
        std::any::TypeId::of::<HttpStreamRecipe>().hash(state);
        self.stream_id.hash(state);
    }

    fn stream(self: Box<Self>, _input: EventStream) -> BoxStream<'static, Message> {
        let receiver_arc = self.receiver;
        let tab_index = self.tab_index;
        futures::stream::unfold(receiver_arc, move |arc| async move {
            let mut receiver = {
                let mut guard = arc.lock().ok()?;
                guard.take()?
            };
            let event = receiver.recv().await?;
            if let Ok(mut guard) = arc.lock() {
                *guard = Some(receiver);
            }
            Some((Message::HttpStreamChunk(tab_index, event), arc))
        })
        .boxed()
    }
}

struct MenuEventRecipe;

impl Recipe for MenuEventRecipe {
    type Output = Message;

    fn hash(&self, state: &mut iced_futures::subscription::Hasher) {
        use std::hash::Hash;
        std::any::TypeId::of::<MenuEventRecipe>().hash(state);
    }

    fn stream(self: Box<Self>, _input: EventStream) -> BoxStream<'static, Message> {
        use std::time::Duration;

        // Two separate intervals:
        // - Menu events: poll every 50ms (low latency for menu interactions)
        // - Mock server logs: poll every 1000ms (logs are low-frequency, no need for sub-second polling)
        let menu_interval = Duration::from_millis(50);
        let log_interval = Duration::from_secs(1);

        futures::stream::unfold(
            (
                tokio::time::Instant::now() + menu_interval,
                tokio::time::Instant::now() + log_interval,
            ),
            move |(next_menu_tick, next_log_tick)| async move {
                let now = tokio::time::Instant::now();

                // Sleep until the earliest tick
                let sleep_dur = std::cmp::min(
                    next_menu_tick.saturating_duration_since(now),
                    next_log_tick.saturating_duration_since(now),
                );
                if !sleep_dur.is_zero() {
                    tokio::time::sleep(sleep_dur).await;
                }
                let now = tokio::time::Instant::now();

                // Check menu events first (higher priority)
                if now >= next_menu_tick {
                    if let Some(msg) = muda::MenuEvent::receiver()
                        .try_recv()
                        .ok()
                        .and_then(|event| crate::ui::menu::handle_menu_event(&event))
                    {
                        return Some((msg, (now + menu_interval, next_log_tick)));
                    }
                }

                // Then check mock server logs (lower frequency)
                if now >= next_log_tick {
                    return Some((
                        Message::PollMockServerLogs,
                        (next_menu_tick, now + log_interval),
                    ));
                }

                // Nothing to do, sleep until next menu tick
                Some((Message::NoOp, (now + menu_interval, next_log_tick)))
            },
        )
        .boxed()
    }
}

struct DevicePollRecipe {
    tab_index: usize,
    device_code: String,
    client_id: String,
    client_secret: String,
    token_url: String,
    interval_secs: u64,
    http_client: Arc<reqwest::Client>,
}

impl Recipe for DevicePollRecipe {
    type Output = Message;

    fn hash(&self, state: &mut iced_futures::subscription::Hasher) {
        use std::hash::Hash;
        std::any::TypeId::of::<DevicePollRecipe>().hash(state);
        self.tab_index.hash(state);
        self.device_code.hash(state);
    }

    fn stream(self: Box<Self>, _input: EventStream) -> BoxStream<'static, Message> {
        let tab_index = self.tab_index;
        let device_code = self.device_code;
        let client_id = self.client_id;
        let client_secret = self.client_secret;
        let token_url = self.token_url;
        let interval = std::time::Duration::from_secs(self.interval_secs.max(5));
        let http_client = self.http_client;

        futures::stream::unfold((), move |()| {
            let device_code = device_code.clone();
            let client_id = client_id.clone();
            let client_secret = client_secret.clone();
            let token_url = token_url.clone();
            let http_client = http_client.clone();
            async move {
                tokio::time::sleep(interval).await;
                let result = crate::data::oauth2::poll_device_token(
                    &http_client,
                    &token_url,
                    &device_code,
                    &client_id,
                    &client_secret,
                )
                .await;
                Some((Message::OAuth2DeviceTokenPoll(tab_index, result), ()))
            }
        })
        .boxed()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Protocol {
    Http,
    WebSocket,
    GraphQL,
    MockServer,
}

impl std::fmt::Display for Protocol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Protocol::Http => write!(f, "HTTP"),
            Protocol::WebSocket => write!(f, "WebSocket"),
            Protocol::GraphQL => write!(f, "GraphQL"),
            Protocol::MockServer => write!(f, "Mock Server"),
        }
    }
}

impl Protocol {
    pub const ALL: [Protocol; 4] = [
        Protocol::Http,
        Protocol::WebSocket,
        Protocol::GraphQL,
        Protocol::MockServer,
    ];
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    Main,
    EnvironmentManager,
    CookieManager,
}

pub fn main() -> iced::Result {
    iced::application(AstraioApp::new, AstraioApp::update, AstraioApp::view)
        .title("Astraio Client")
        .subscription(AstraioApp::subscription)
        .theme(AstraioApp::theme)
        .font(iced_fonts::LUCIDE_FONT_BYTES)
        .run()
}

pub(crate) struct AstraioApp {
    pub(crate) request_tabs: Vec<HttpRequestView>,
    pub(crate) active_request_tab_index: usize,
    pub(crate) http_client: Arc<reqwest::Client>,
    pub(crate) custom_clients: HashMap<String, (Arc<reqwest::Client>, std::time::Instant)>,
    pub(crate) cookie_jar: Arc<std::sync::Mutex<CookieJar>>,
    pub(crate) db_conn: rusqlite::Connection,
    pub(crate) environments: Vec<Environment>,
    pub(crate) active_environment: Option<Environment>,
    pub(crate) env_manager_view: EnvironmentManagerView,
    pub(crate) history_view: HistoryView,
    pub(crate) collection_view: CollectionView,
    pub(crate) websocket_view: WebSocketView,
    pub(crate) graphql_view: GraphQLView,
    pub(crate) mock_server_view: crate::ui::views::mock_server_view::MockServerView,
    pub(crate) mock_server_handles:
        std::collections::HashMap<i32, crate::protocols::mock_server::MockServerHandle>,
    pub(crate) active_protocol: Protocol,
    pub(crate) current_view: View,
    pub(crate) show_history: bool,
    pub(crate) show_collections: bool,
    pub(crate) show_env_info: bool,
    pub(crate) ws_sender: Option<WsSender>,
    pub(crate) ws_receiver: Option<Arc<Mutex<Option<mpsc::UnboundedReceiver<WsEvent>>>>>,
    pub(crate) ws_shutdown: Option<mpsc::UnboundedSender<()>>,
    pub(crate) ws_write_handle: Option<Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>>,
    pub(crate) ws_read_handle: Option<Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>>,
    pub(crate) ws_ping_handle: Option<Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>>,
    pub(crate) ws_connection_id: u64,
    pub(crate) http_stream_receivers: HashMap<usize, (u64, HttpStreamReceiver)>,
    pub(crate) http_stream_id: u64,
    pub(crate) toast_manager: ToastManager,
    pub(crate) dark_mode: bool,
    pub(crate) secret_store: crate::services::secret_store::SecretStore,
    pub(crate) global_config: crate::http_client::config::GlobalConfig,
    pub(crate) main_window_id: Option<iced::window::Id>,
    pub(crate) cookie_manager_view: crate::ui::views::cookie_manager::CookieManagerView,
    pub(crate) show_collection_runner: bool,
    pub(crate) collection_runner_state:
        Option<crate::ui::views::collection_runner::CollectionRunnerState>,
}

impl Drop for AstraioApp {
    fn drop(&mut self) {
        self.cleanup();
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    HttpRequestViewMsg(usize, http_request_view::Message),
    AddRequestTab,
    CloseRequestTab(usize),
    CloseActiveRequestTab,
    NoOp,
    SelectRequestTab(usize),
    PrevRequestTab,
    NextRequestTab,
    EnvManagerMsg(environment_manager::Message),
    EnvFileLoaded(Option<Vec<(String, String)>>),
    EnvFileExported(Option<String>),
    SelectEnvironment(i32),
    SwitchView(View),
    ToggleEnvironmentManager,
    HistoryMsg(history_view::Message),
    HistoryExportComplete(Option<String>),
    ToggleHistory,
    CollectionMsg(collection_view::Message),
    ToggleCollections,
    ToggleEnvInfo,
    ToggleTheme,
    WebSocketMsg(websocket_view::Message),
    GraphQLMsg(graphql_view::Message),
    WsEvent(crate::protocols::websocket::WsEvent),
    WsConnected(
        WsSender,
        Arc<Mutex<Option<mpsc::UnboundedReceiver<WsEvent>>>>,
        Option<mpsc::UnboundedSender<()>>,
        Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
        Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
        Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
    ),
    SelectProtocol(Protocol),
    OAuth2StartAuth(usize),
    OAuth2AuthComplete(
        usize,
        Result<String, crate::error::AppError>,
        Option<String>,
    ),
    OAuth2TokenReceived(
        usize,
        Result<crate::data::oauth2::OAuth2TokenResponse, crate::error::AppError>,
    ),
    OAuth2RefreshToken(usize),
    OAuth2StartDeviceAuth(usize),
    OAuth2DeviceAuthReceived(
        usize,
        Result<crate::data::oauth2::DeviceAuthorizationResponse, crate::error::AppError>,
    ),
    OAuth2DeviceTokenPoll(
        usize,
        Result<crate::data::oauth2::DeviceTokenResponse, crate::error::AppError>,
    ),
    OAuth2AutoPollToggle(usize, bool),
    ToggleResponseSearch,
    WsSendFromKeyboard,
    SendActiveRequest,
    EscapePressed,
    ClearKeychainSecrets,
    KeychainCleared(Result<u32, crate::error::AppError>),
    ClearCookies,
    ClearDomainCookies(String),
    DeleteCookie(String, String, String),
    SaveCookieEdit(String, String, String, String),
    ImportCookies,
    ImportCookiesData(Option<String>),
    ExportCookies,
    ExportCookiesComplete(Option<String>),
    ToggleSidebar,
    ShowAbout,
    Quit,
    WindowOpened(iced::window::Id),
    MockServerMsg(crate::ui::views::mock_server_view::Message),
    MockServerStarted(i32, crate::protocols::mock_server::MockServerHandle, u16),
    MockServerStartError(i32, String),
    PollMockServerLogs,
    GraphQLOAuth2StartAuth,
    GraphQLOAuth2AuthComplete(Result<String, crate::error::AppError>, Option<String>),
    GraphQLOAuth2TokenReceived(
        Result<crate::data::oauth2::OAuth2TokenResponse, crate::error::AppError>,
    ),
    GraphQLOAuth2RefreshToken,
    GraphQLOAuth2StartDeviceAuth,
    GraphQLOAuth2DeviceAuthReceived(
        Result<crate::data::oauth2::DeviceAuthorizationResponse, crate::error::AppError>,
    ),
    GraphQLOAuth2DeviceTokenPoll(
        Result<crate::data::oauth2::DeviceTokenResponse, crate::error::AppError>,
    ),
    GraphQLOAuth2AutoPollToggle(bool),
    HttpStreamChunk(usize, crate::http_client::response::HttpStreamEvent),
    CookieManagerMsg(crate::ui::views::cookie_manager::Message),
    ToggleCookieManager,
    CollectionRunnerMsg(crate::ui::views::collection_runner::Message),
}

impl AstraioApp {
    fn new() -> (Self, Task<Message>) {
        let (db_conn, environments) = match database::init() {
            Ok(conn) => {
                let envs =
                    crate::services::environment_service::get_all(&conn).unwrap_or_else(|e| {
                        log::error!("Failed to load environments: {e}");
                        Vec::new()
                    });
                (conn, envs)
            }
            Err(e) => {
                log::error!("Failed to initialize database: {e}");
                let conn = rusqlite::Connection::open_in_memory()
                    .expect("In-memory DB should always work");
                if let Err(schema_err) = database::init_schema(&conn) {
                    log::error!("Failed to init in-memory schema: {schema_err}");
                }
                (conn, Vec::new())
            }
        };

        let history =
            crate::services::history_service::get_all(&db_conn, 200).unwrap_or_else(|e| {
                log::error!("Failed to load history: {e}");
                Vec::new()
            });
        let collections =
            crate::services::collection_service::get_all(&db_conn).unwrap_or_else(|e| {
                log::error!("Failed to load collections: {e}");
                Vec::new()
            });

        let mut cv = CollectionView::new();
        cv.sync_collections(&collections);

        let mock_servers =
            crate::services::mock_server_service::get_all(&db_conn).unwrap_or_else(|e| {
                log::warn!("Failed to load mock servers: {e}");
                Vec::new()
            });

        let secret_store = crate::services::secret_store::SecretStore::new();
        match crate::services::secret_store::migrate_plaintext_tokens_to_keyring(
            &secret_store,
            &db_conn,
        ) {
            Ok(0) => {}
            Ok(n) => log::info!("Migrated {n} plaintext tokens to OS keyring"),
            Err(e) => log::warn!("Keyring migration skipped: {e}"),
        }

        // Load theme preference from database
        let dark_mode = crate::persistence::database::get_app_setting(&db_conn, "theme")
            .is_none_or(|v| v != "light");

        // Load global config
        let global_config = crate::http_client::config::GlobalConfig::load(&db_conn);

        let sessions = crate::persistence::database::load_sessions(&db_conn).unwrap_or_else(|e| {
            log::warn!("Failed to load sessions: {e}");
            Vec::new()
        });

        let default_tab = HttpRequestView {
            request_config: global_config.request_config.clone(),
            sessions: sessions.clone(),
            ..HttpRequestView::default()
        };

        let app = Self {
            request_tabs: vec![default_tab],
            active_request_tab_index: 0,
            http_client: Arc::new(
                reqwest::Client::builder()
                    .cookie_store(true)
                    .build()
                    .unwrap_or_else(|_| reqwest::Client::new()),
            ),
            custom_clients: HashMap::new(),
            cookie_jar: Arc::new(std::sync::Mutex::new(
                crate::persistence::database::load_cookies(&db_conn).unwrap_or_else(|e| {
                    log::warn!("Failed to load cookies from SQLite: {e}");
                    CookieJar::new()
                }),
            )),
            db_conn,
            environments: environments.clone(),
            active_environment: None,
            env_manager_view: EnvironmentManagerView::new(environments),
            history_view: {
                let mut hv = HistoryView::new();
                hv.entries = history;
                hv
            },
            collection_view: cv,
            websocket_view: WebSocketView::new(),
            graphql_view: GraphQLView::default(),
            mock_server_view: {
                let mut mv = crate::ui::views::mock_server_view::MockServerView::default();
                mv.sync_servers(&mock_servers);
                mv
            },
            mock_server_handles: std::collections::HashMap::new(),
            active_protocol: Protocol::Http,
            current_view: View::Main,
            show_history: false,
            show_collections: false,
            show_env_info: false,
            ws_sender: None,
            ws_receiver: None,
            ws_shutdown: None,
            ws_write_handle: None,
            ws_read_handle: None,
            ws_ping_handle: None,
            ws_connection_id: 0,
            http_stream_receivers: HashMap::new(),
            http_stream_id: 0,
            toast_manager: ToastManager::new(),
            dark_mode,
            secret_store,
            global_config,
            main_window_id: None,
            cookie_manager_view: crate::ui::views::cookie_manager::CookieManagerView::default(),
            show_collection_runner: false,
            collection_runner_state: None,
        };
        (app, Task::none())
    }

    fn cleanup(&mut self) {
        // Shutdown active WebSocket connections gracefully
        if let Some(shutdown_tx) = self.ws_shutdown.take() {
            let _ = shutdown_tx.send(());
        }
        // Abort any lingering WebSocket tasks
        if let Some(handle) = self.ws_write_handle.take() {
            if let Ok(mut guard) = handle.lock() {
                if let Some(h) = guard.take() {
                    h.abort();
                }
            }
        }
        if let Some(handle) = self.ws_read_handle.take() {
            if let Ok(mut guard) = handle.lock() {
                if let Some(h) = guard.take() {
                    h.abort();
                }
            }
        }
        if let Some(handle) = self.ws_ping_handle.take() {
            if let Ok(mut guard) = handle.lock() {
                if let Some(h) = guard.take() {
                    h.abort();
                }
            }
        }
        // Persist cookies on shutdown
        if let Ok(jar) = self.cookie_jar.lock() {
            if let Err(e) = crate::persistence::database::save_cookies(&self.db_conn, &jar) {
                log::warn!("Failed to persist cookies on shutdown: {e}");
            }
        }
        // Shutdown mock servers
        for (id, handle) in self.mock_server_handles.drain() {
            crate::protocols::mock_server::stop_mock_server(handle);
            log::info!("[Mock] Stopped mock server id={id}");
        }
        log::info!("Astraio cleanup complete");
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        self.toast_manager.clean_expired();
        match message {
            Message::HttpRequestViewMsg(index, msg) => {
                super::handlers::http_request::handle_http_request_msg(self, index, msg)
            }
            Message::HttpStreamChunk(tab_index, event) => {
                use crate::http_client::response::HttpStreamEvent;
                if let Some(view) = self.request_tabs.get_mut(tab_index) {
                    view.update(http_request_view::Message::StreamEvent(
                        tab_index,
                        event.clone(),
                    ));

                    if matches!(
                        event,
                        HttpStreamEvent::StreamComplete { .. } | HttpStreamEvent::StreamError(_)
                    ) {
                        self.http_stream_receivers.remove(&tab_index);
                    }
                }
                Task::none()
            }
            Message::AddRequestTab => {
                let mut new_view = HttpRequestView {
                    request_config: self.global_config.request_config.clone(),
                    sessions: crate::persistence::database::load_sessions(&self.db_conn)
                        .unwrap_or_default(),
                    ..HttpRequestView::default()
                };
                if let Some(env) = &self.active_environment {
                    if let Some(url) = &env.default_endpoint {
                        if !url.is_empty() {
                            new_view.url_input = url.clone();
                        }
                    }
                }
                self.request_tabs.push(new_view);
                self.active_request_tab_index = self.request_tabs.len() - 1;
                Task::none()
            }
            Message::CloseRequestTab(index) => {
                if self.request_tabs.len() > 1 {
                    self.request_tabs.remove(index);
                    self.http_stream_receivers.remove(&index);
                    if self.active_request_tab_index >= self.request_tabs.len() {
                        self.active_request_tab_index = self.request_tabs.len() - 1;
                    }
                    self.sync_cookie_data_to_tabs();
                }
                Task::none()
            }
            Message::CloseActiveRequestTab => {
                if self.request_tabs.len() > 1 {
                    let index = self.active_request_tab_index;
                    self.request_tabs.remove(index);
                    self.http_stream_receivers.remove(&index);
                    if self.active_request_tab_index >= self.request_tabs.len() {
                        self.active_request_tab_index = self.request_tabs.len() - 1;
                    }
                    self.sync_cookie_data_to_tabs();
                }
                Task::none()
            }
            Message::NoOp => Task::none(),
            Message::SelectRequestTab(index) => {
                self.active_request_tab_index = index;
                self.sync_cookie_data_to_tabs();
                Task::none()
            }
            Message::PrevRequestTab => {
                if !self.request_tabs.is_empty() {
                    self.active_request_tab_index =
                        (self.active_request_tab_index + self.request_tabs.len() - 1)
                            % self.request_tabs.len();
                    self.sync_cookie_data_to_tabs();
                }
                Task::none()
            }
            Message::NextRequestTab => {
                if !self.request_tabs.is_empty() {
                    self.active_request_tab_index =
                        (self.active_request_tab_index + 1) % self.request_tabs.len();
                    self.sync_cookie_data_to_tabs();
                }
                Task::none()
            }
            Message::EnvManagerMsg(msg) => super::handlers::environment::handle_message(self, msg),
            Message::EnvFileLoaded(vars) => {
                super::handlers::environment::handle_file_loaded(self, vars)
            }
            Message::EnvFileExported(content) => {
                if let Some(content) = content {
                    self.toast_manager
                        .success(format!("Exported .env file ({} bytes)", content.len()));
                }
                Task::none()
            }
            Message::SelectEnvironment(id) => {
                self.active_environment = self.environments.iter().find(|e| e.id == id).cloned();
                Task::none()
            }
            Message::SwitchView(view) => {
                self.current_view = view;
                Task::none()
            }
            Message::ToggleEnvironmentManager => {
                self.current_view = match self.current_view {
                    View::EnvironmentManager => View::Main,
                    View::Main => View::EnvironmentManager,
                    View::CookieManager => View::CookieManager,
                };
                Task::none()
            }
            Message::ToggleHistory => {
                self.show_history = !self.show_history;
                Task::none()
            }
            Message::ToggleCollections => {
                self.show_collections = !self.show_collections;
                if self.show_collections {
                    let cols = crate::services::collection_service::get_all(&self.db_conn)
                        .unwrap_or_else(|e| {
                            log::error!("Failed to refresh collections: {e}");
                            Vec::new()
                        });
                    self.collection_view.sync_collections(&cols);
                }
                Task::none()
            }
            Message::ToggleEnvInfo => {
                self.show_env_info = !self.show_env_info;
                Task::none()
            }
            Message::ToggleTheme => {
                self.dark_mode = !self.dark_mode;
                // Persist theme preference
                let theme_value = if self.dark_mode { "dark" } else { "light" };
                let _ = crate::persistence::database::set_app_setting(
                    &self.db_conn,
                    "theme",
                    theme_value,
                );
                Task::none()
            }
            Message::CollectionMsg(msg) => super::handlers::collection::handle_message(self, msg),
            Message::HistoryMsg(msg) => super::handlers::history::handle_message(self, msg),
            Message::HistoryExportComplete(result) => {
                if let Some(msg) = result {
                    if msg.contains("failed") || msg.contains("cancelled") {
                        self.toast_manager.warning(msg);
                    } else {
                        self.toast_manager.success(msg);
                    }
                }
                Task::none()
            }
            Message::SelectProtocol(protocol) => {
                self.active_protocol = protocol;
                Task::none()
            }
            Message::WsEvent(event) => super::handlers::websocket::handle_ws_event(self, event),
            Message::WebSocketMsg(msg) => super::handlers::websocket::handle_message(self, msg),
            Message::GraphQLMsg(msg) => super::handlers::graphql::handle_message(self, msg),
            Message::MockServerMsg(msg) => super::handlers::mock_server::handle_message(self, msg),
            Message::MockServerStarted(id, handle, actual_port) => {
                self.mock_server_handles.insert(id, handle);
                self.mock_server_view.statuses.insert(
                    id,
                    crate::protocols::mock_server::MockServerStatus::Running { actual_port },
                );
                self.toast_manager
                    .success(format!("Mock server running on port {actual_port}"));
                Task::none()
            }
            Message::MockServerStartError(id, error) => {
                self.mock_server_view.statuses.insert(
                    id,
                    crate::protocols::mock_server::MockServerStatus::Error(error.clone()),
                );
                self.toast_manager
                    .error(format!("Mock server error: {error}"));
                Task::none()
            }
            Message::PollMockServerLogs => {
                for handle in self.mock_server_handles.values() {
                    if let Ok(mut rx) = handle.log_rx.try_lock() {
                        while let Ok(log) = rx.try_recv() {
                            self.mock_server_view.logs.push(log);
                        }
                    }
                }
                Task::none()
            }
            Message::GraphQLOAuth2StartAuth => {
                super::handlers::oauth2::handle_graphql_start_auth(self)
            }
            Message::GraphQLOAuth2AuthComplete(result, pkce_verifier) => {
                super::handlers::oauth2::handle_graphql_auth_complete(self, result, pkce_verifier)
            }
            Message::GraphQLOAuth2TokenReceived(result) => {
                super::handlers::oauth2::handle_graphql_token_received(self, result)
            }
            Message::GraphQLOAuth2RefreshToken => {
                super::handlers::oauth2::handle_graphql_refresh_token(self)
            }
            Message::GraphQLOAuth2StartDeviceAuth => {
                super::handlers::oauth2::handle_graphql_start_device_auth(self)
            }
            Message::GraphQLOAuth2DeviceAuthReceived(result) => {
                super::handlers::oauth2::handle_graphql_device_auth_received(self, result)
            }
            Message::GraphQLOAuth2DeviceTokenPoll(result) => {
                super::handlers::oauth2::handle_graphql_device_token_poll(self, result)
            }
            Message::GraphQLOAuth2AutoPollToggle(enabled) => {
                super::handlers::oauth2::handle_graphql_auto_poll_toggle(self, enabled)
            }
            Message::WsConnected(
                sender,
                receiver_arc,
                shutdown_tx,
                write_handle,
                read_handle,
                ping_handle,
            ) => {
                super::handlers::websocket::handle_ws_connected(
                    self,
                    sender,
                    receiver_arc,
                    shutdown_tx,
                    write_handle,
                    read_handle,
                    ping_handle,
                );
                Task::none()
            }
            Message::OAuth2StartAuth(index) => {
                super::handlers::oauth2::handle_start_auth(self, index)
            }
            Message::OAuth2AuthComplete(index, result, pkce_verifier) => {
                super::handlers::oauth2::handle_auth_complete(self, index, result, pkce_verifier)
            }
            Message::OAuth2TokenReceived(index, result) => {
                super::handlers::oauth2::handle_token_received(self, index, result)
            }
            Message::OAuth2RefreshToken(index) => {
                super::handlers::oauth2::handle_refresh_token(self, index)
            }
            Message::OAuth2StartDeviceAuth(index) => {
                super::handlers::oauth2::handle_start_device_auth(self, index)
            }
            Message::OAuth2DeviceAuthReceived(index, result) => {
                super::handlers::oauth2::handle_device_auth_received(self, index, result)
            }
            Message::OAuth2DeviceTokenPoll(index, result) => {
                super::handlers::oauth2::handle_device_token_poll(self, index, result)
            }
            Message::OAuth2AutoPollToggle(index, enabled) => {
                super::handlers::oauth2::handle_auto_poll_toggle(self, index, enabled)
            }
            Message::ToggleResponseSearch => {
                if let Some(view) = self.request_tabs.get_mut(self.active_request_tab_index) {
                    view.update(http_request_view::Message::ToggleResponseSearch);
                }
                Task::none()
            }
            Message::WsSendFromKeyboard => {
                Self::send_ws_message(&mut self.websocket_view);
                Task::none()
            }
            Message::SendActiveRequest => {
                match self.active_protocol {
                    Protocol::WebSocket => {
                        Self::send_ws_message(&mut self.websocket_view);
                    }
                    Protocol::GraphQL => {
                        return super::handlers::graphql::handle_message(
                            self,
                            graphql_view::Message::SendRequest,
                        );
                    }
                    Protocol::Http => {
                        return super::handlers::http_request::handle_http_request_msg(
                            self,
                            self.active_request_tab_index,
                            http_request_view::Message::SendRequest,
                        );
                    }
                    Protocol::MockServer => {}
                }
                Task::none()
            }
            Message::EscapePressed => {
                if self.show_env_info {
                    self.show_env_info = false;
                } else if let Some(view) = self.request_tabs.get(self.active_request_tab_index) {
                    if view.show_response_search {
                        return super::handlers::http_request::handle_http_request_msg(
                            self,
                            self.active_request_tab_index,
                            http_request_view::Message::ToggleResponseSearch,
                        );
                    } else if view.show_snippets {
                        return super::handlers::http_request::handle_http_request_msg(
                            self,
                            self.active_request_tab_index,
                            http_request_view::Message::ShowSnippets,
                        );
                    }
                }
                Task::none()
            }
            Message::ClearKeychainSecrets => {
                let store = self.secret_store.clone();
                let conn = &self.db_conn;
                let mut identifiers = Vec::new();

                if let Ok(mut stmt) = conn.prepare(
                    "SELECT id, collection_id FROM collection_requests WHERE auth_type = 'oauth2'",
                ) {
                    if let Ok(rows) =
                        stmt.query_map([], |row| Ok((row.get::<_, i32>(0)?, row.get::<_, i32>(1)?)))
                    {
                        for row in rows.flatten() {
                            identifiers.push(format!("col_{}_{}", row.1, row.0));
                        }
                    }
                }

                if let Ok(mut stmt) = conn
                    .prepare("SELECT id FROM request_history WHERE request_data LIKE '%OAuth2%'")
                {
                    if let Ok(rows) = stmt.query_map([], |row| row.get::<_, i32>(0)) {
                        for row in rows.flatten() {
                            identifiers.push(format!("hist_{row}"));
                        }
                    }
                }

                Task::perform(
                    async move {
                        let mut total = 0u32;
                        for identifier in &identifiers {
                            let _ = store.delete_oauth2_tokens(identifier);
                            total += 1;
                        }
                        total
                    },
                    |count| Message::KeychainCleared(Ok(count)),
                )
            }
            Message::KeychainCleared(result) => {
                match result {
                    Ok(count) => {
                        self.toast_manager
                            .success(format!("Cleared {count} keychain entries"));
                    }
                    Err(e) => {
                        self.toast_manager
                            .error(format!("Failed to clear keychain: {e}"));
                    }
                }
                Task::none()
            }
            Message::ClearCookies => {
                if let Ok(mut jar) = self.cookie_jar.lock() {
                    jar.clear();
                } else {
                    log::error!("Failed to acquire cookie_jar lock for ClearCookies");
                }
                if let Err(e) = crate::persistence::database::clear_cookies_db(&self.db_conn) {
                    log::warn!("Failed to clear cookies from DB: {e}");
                }
                for tab in &mut self.request_tabs {
                    tab.cookie_count = 0;
                    tab.cookie_domain_count = 0;
                    tab.cookie_domains.clear();
                    tab.cookie_domain_cookies.clear();
                }
                self.toast_manager.success("Cookies cleared".to_string());
                Task::none()
            }
            Message::ClearDomainCookies(domain) => {
                if let Ok(mut jar) = self.cookie_jar.lock() {
                    jar.clear_domain(&domain);
                } else {
                    log::error!("Failed to acquire cookie_jar lock for ClearDomainCookies");
                }
                if let Err(e) =
                    crate::persistence::database::clear_domain_cookies_db(&self.db_conn, &domain)
                {
                    log::warn!("Failed to clear domain cookies from DB: {e}");
                }
                self.sync_cookie_data_to_tabs();
                self.toast_manager
                    .success(format!("Cookies for {domain} cleared"));
                Task::none()
            }
            Message::DeleteCookie(domain, name, path) => {
                if let Ok(mut jar) = self.cookie_jar.lock() {
                    jar.remove_cookie(&domain, &name, &path);
                } else {
                    log::error!("Failed to acquire cookie_jar lock for DeleteCookie");
                }
                if let Err(e) = crate::persistence::database::delete_cookie_db(
                    &self.db_conn,
                    &domain,
                    &name,
                    &path,
                ) {
                    log::warn!("Failed to delete cookie from DB: {e}");
                }
                self.sync_cookie_data_to_tabs();
                Task::none()
            }
            Message::SaveCookieEdit(domain, name, path, new_value) => {
                if let Ok(mut jar) = self.cookie_jar.lock() {
                    if let Some(cookies) = jar.cookies_for_domain_mut(&domain) {
                        for c in cookies.iter_mut() {
                            if c.name == name && c.path == path {
                                c.value = new_value.clone();
                                break;
                            }
                        }
                    }
                } else {
                    log::error!("Failed to acquire cookie_jar lock for SaveCookieEdit");
                }
                if let Err(e) = crate::persistence::database::update_cookie_value_db(
                    &self.db_conn,
                    &domain,
                    &name,
                    &path,
                    &new_value,
                ) {
                    log::warn!("Failed to update cookie in DB: {e}");
                }
                self.sync_cookie_data_to_tabs();
                Task::none()
            }
            Message::ImportCookies => Task::perform(
                async {
                    rfd::AsyncFileDialog::new()
                        .add_filter("Cookie files", &["txt", "json", "cookie", "cookies"])
                        .pick_file()
                        .await
                        .map(|f| f.path().to_path_buf())
                        .and_then(|p| std::fs::read_to_string(p).ok())
                },
                Message::ImportCookiesData,
            ),
            Message::ImportCookiesData(content) => {
                if let Some(content) = content {
                    let new_jar = if let Ok(jar) = crate::cookie::CookieJar::from_json(&content) {
                        jar
                    } else if let Ok(jar) = crate::cookie::CookieJar::from_netscape(&content) {
                        jar
                    } else {
                        self.toast_manager
                            .error("Failed to parse cookie file".to_string());
                        return Task::none();
                    };
                    {
                        if let Ok(mut jar) = self.cookie_jar.lock() {
                            for cookie in new_jar.all_cookies() {
                                jar.insert(cookie.clone());
                            }
                        }
                    }
                    if let Ok(jar) = self.cookie_jar.lock() {
                        if let Err(e) =
                            crate::persistence::database::save_cookies(&self.db_conn, &jar)
                        {
                            log::warn!("Failed to persist imported cookies: {e}");
                        }
                    } else {
                        log::error!(
                            "Failed to acquire cookie_jar lock for persisting imported cookies"
                        );
                    }
                    self.sync_cookie_data_to_tabs();
                    self.toast_manager
                        .success("Cookies imported successfully".to_string());
                }
                Task::none()
            }
            Message::ExportCookies => {
                let content = match self.cookie_jar.lock() {
                    Ok(jar) => jar.to_netscape(),
                    Err(e) => {
                        log::error!("Failed to acquire cookie_jar lock for export: {e}");
                        return Task::none();
                    }
                };
                Task::perform(
                    async move {
                        rfd::AsyncFileDialog::new()
                            .add_filter("Cookie files", &["txt"])
                            .set_file_name("cookies.txt")
                            .save_file()
                            .await
                            .and_then(|f| {
                                std::fs::write(f.path(), &content).ok()?;
                                Some(())
                            })
                    },
                    |result| {
                        Message::ExportCookiesComplete(result.map(|()| "Exported".to_string()))
                    },
                )
            }
            Message::ExportCookiesComplete(result) => {
                if let Some(msg) = result {
                    self.toast_manager.success(msg);
                }
                Task::none()
            }
            Message::ToggleSidebar => {
                self.show_collections = !self.show_collections;
                Task::none()
            }
            Message::ShowAbout => {
                self.toast_manager.info("Astraio Client v0.5.0");
                Task::none()
            }
            Message::Quit => {
                if let Some(id) = self.main_window_id {
                    iced::window::close(id)
                } else {
                    Task::none()
                }
            }
            Message::WindowOpened(id) => {
                if self.main_window_id.is_none() {
                    self.main_window_id = Some(id);
                }
                #[cfg(target_os = "macos")]
                {
                    crate::ui::menu::attach_macos();
                }
                Task::none()
            }
            Message::CookieManagerMsg(msg) => {
                use crate::ui::views::cookie_manager::CookieManagerAction;
                if let Some(action) = self.cookie_manager_view.update(msg) {
                    match action {
                        CookieManagerAction::DeleteCookie(domain, name, path) => {
                            if let Ok(mut jar) = self.cookie_jar.lock() {
                                jar.remove_cookie(&domain, &name, &path);
                            }
                            if let Err(e) = crate::persistence::database::delete_cookie_db(
                                &self.db_conn,
                                &domain,
                                &name,
                                &path,
                            ) {
                                log::warn!("Failed to delete cookie from DB: {e}");
                            }
                            self.sync_cookie_data_to_tabs();
                            self.toast_manager.success("Cookie deleted");
                        }
                        CookieManagerAction::ClearDomain(domain) => {
                            if let Ok(mut jar) = self.cookie_jar.lock() {
                                jar.clear_domain(&domain);
                            }
                            if let Err(e) = crate::persistence::database::clear_domain_cookies_db(
                                &self.db_conn,
                                &domain,
                            ) {
                                log::warn!("Failed to clear domain cookies from DB: {e}");
                            }
                            self.sync_cookie_data_to_tabs();
                            self.toast_manager
                                .success(format!("Cookies for {domain} cleared"));
                        }
                        CookieManagerAction::ClearAll => {
                            if let Ok(mut jar) = self.cookie_jar.lock() {
                                jar.clear();
                            }
                            if let Err(e) =
                                crate::persistence::database::clear_cookies_db(&self.db_conn)
                            {
                                log::warn!("Failed to clear cookies from DB: {e}");
                            }
                            for tab in &mut self.request_tabs {
                                tab.cookie_count = 0;
                                tab.cookie_domain_count = 0;
                                tab.cookie_domains.clear();
                                tab.cookie_domain_cookies.clear();
                            }
                            self.toast_manager.success("All cookies cleared");
                        }
                        CookieManagerAction::SaveEdit(domain, name, path, new_value) => {
                            if let Ok(mut jar) = self.cookie_jar.lock() {
                                if let Some(cookies) = jar.cookies_for_domain_mut(&domain) {
                                    for c in cookies.iter_mut() {
                                        if c.name == name && c.path == path {
                                            c.value = new_value.clone();
                                            break;
                                        }
                                    }
                                }
                            }
                            if let Err(e) = crate::persistence::database::update_cookie_value_db(
                                &self.db_conn,
                                &domain,
                                &name,
                                &path,
                                &new_value,
                            ) {
                                log::warn!("Failed to update cookie in DB: {e}");
                            }
                            self.sync_cookie_data_to_tabs();
                            self.toast_manager.success("Cookie updated");
                        }
                        CookieManagerAction::ImportCookies => {
                            return Task::perform(
                                async {
                                    rfd::AsyncFileDialog::new()
                                        .add_filter(
                                            "Cookie files",
                                            &["txt", "json", "cookie", "cookies"],
                                        )
                                        .pick_file()
                                        .await
                                        .map(|f| f.path().to_path_buf())
                                        .and_then(|p| std::fs::read_to_string(p).ok())
                                },
                                Message::ImportCookiesData,
                            );
                        }
                        CookieManagerAction::ExportCookies => {
                            let content = match self.cookie_jar.lock() {
                                Ok(jar) => jar.to_netscape(),
                                Err(e) => {
                                    log::error!(
                                        "Failed to acquire cookie_jar lock for export: {e}"
                                    );
                                    return Task::none();
                                }
                            };
                            return Task::perform(
                                async move {
                                    rfd::AsyncFileDialog::new()
                                        .add_filter("Cookie files", &["txt"])
                                        .set_file_name("cookies.txt")
                                        .save_file()
                                        .await
                                        .and_then(|f| {
                                            std::fs::write(f.path(), &content).ok()?;
                                            Some(())
                                        })
                                },
                                |result| {
                                    Message::ExportCookiesComplete(
                                        result.map(|()| "Exported".to_string()),
                                    )
                                },
                            );
                        }
                    }
                    // Re-sync after any action
                    if let Ok(jar) = self.cookie_jar.lock() {
                        self.cookie_manager_view.sync_from_jar(&jar);
                    }
                }
                Task::none()
            }
            Message::ToggleCookieManager => {
                self.current_view = match self.current_view {
                    View::CookieManager => View::Main,
                    View::Main => View::CookieManager,
                    View::EnvironmentManager => View::CookieManager,
                };
                if self.current_view == View::CookieManager {
                    if let Ok(jar) = self.cookie_jar.lock() {
                        self.cookie_manager_view.sync_from_jar(&jar);
                    }
                }
                Task::none()
            }
            Message::CollectionRunnerMsg(msg) => {
                super::handlers::collection_runner::handle_message(self, msg)
            }
        }
    }

    pub(crate) fn sync_cookie_data_to_tabs(&mut self) {
        let jar = match self.cookie_jar.lock() {
            Ok(jar) => jar,
            Err(e) => {
                log::error!("Failed to acquire cookie_jar lock for sync: {e}");
                return;
            }
        };

        let domains: Vec<(String, usize)> = jar
            .domains()
            .into_iter()
            .map(|(d, c)| (d.to_string(), c))
            .collect();
        let total = jar.total_count();
        let domain_count = jar.domain_count();

        let active_idx = self.active_request_tab_index;

        // Only build cookie snapshots for the active tab
        let active_cookies: Option<Vec<CookieSnapshot>> = if active_idx < self.request_tabs.len() {
            let mut all_cookies = Vec::with_capacity(total);
            for (d, _) in &domains {
                for c in jar.cookies_for_domain(d) {
                    all_cookies.push(CookieSnapshot {
                        name: c.name.clone(),
                        value: c.value.clone(),
                        domain: c.domain.clone(),
                        path: c.path.clone(),
                        secure: c.secure,
                        http_only: c.http_only,
                        same_site: c.same_site.to_string(),
                        expires: c.expires.clone(),
                    });
                }
            }
            Some(all_cookies)
        } else {
            None
        };
        drop(jar);

        // Update only the active tab with full data; other tabs only get counts
        for (i, tab) in self.request_tabs.iter_mut().enumerate() {
            tab.cookie_count = total;
            tab.cookie_domain_count = domain_count;
            if i == active_idx {
                if let Some(ref cookies) = active_cookies {
                    tab.cookie_domains = domains.clone();
                    tab.cookie_domain_cookies = cookies.clone();
                }
            } else {
                // Non-active tabs: clear expensive data, keep only counts
                tab.cookie_domains.clear();
                tab.cookie_domain_cookies.clear();
            }
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        let ws_subscription = if let Some(receiver_arc) = &self.ws_receiver {
            from_recipe(WsRecipe {
                receiver: receiver_arc.clone(),
                connection_id: self.ws_connection_id,
            })
        } else {
            Subscription::none()
        };

        let keyboard_subscription = iced::event::listen_with(|event, status, _window| {
            // Only handle events that no widget claimed (Ignored)
            // This lets text_input handle copy/paste/select-all natively
            if status != iced::event::Status::Ignored {
                return None;
            }

            if let iced::event::Event::Keyboard(iced::keyboard::Event::KeyPressed {
                key,
                modifiers,
                ..
            }) = event
            {
                if modifiers.control() || modifiers.command() {
                    match key {
                        iced::keyboard::Key::Character(ref c)
                            if c.as_ref() == "n" || c.as_ref() == "t" =>
                        {
                            Some(Message::AddRequestTab)
                        }
                        iced::keyboard::Key::Character(ref c) if c.as_ref() == "w" => {
                            Some(Message::CloseActiveRequestTab)
                        }
                        iced::keyboard::Key::Character(ref c) if c.as_ref() == "d" => {
                            Some(Message::ToggleTheme)
                        }
                        iced::keyboard::Key::Named(iced::keyboard::key::Named::ArrowLeft) => {
                            Some(Message::PrevRequestTab)
                        }
                        iced::keyboard::Key::Named(iced::keyboard::key::Named::ArrowRight) => {
                            Some(Message::NextRequestTab)
                        }
                        iced::keyboard::Key::Character(ref c) if c.as_ref() == "1" => {
                            Some(Message::SelectRequestTab(0))
                        }
                        iced::keyboard::Key::Character(ref c) if c.as_ref() == "2" => {
                            Some(Message::SelectRequestTab(1))
                        }
                        iced::keyboard::Key::Character(ref c) if c.as_ref() == "3" => {
                            Some(Message::SelectRequestTab(2))
                        }
                        iced::keyboard::Key::Character(ref c) if c.as_ref() == "4" => {
                            Some(Message::SelectRequestTab(3))
                        }
                        iced::keyboard::Key::Character(ref c) if c.as_ref() == "5" => {
                            Some(Message::SelectRequestTab(4))
                        }
                        iced::keyboard::Key::Character(ref c) if c.as_ref() == "f" => {
                            Some(Message::ToggleResponseSearch)
                        }
                        iced::keyboard::Key::Character(ref c) if c.as_ref() == "e" => {
                            Some(Message::ToggleEnvironmentManager)
                        }
                        iced::keyboard::Key::Character(ref c) if c.as_ref() == "s" => {
                            Some(Message::CollectionMsg(
                                super::views::collection_view::Message::SaveCurrentRequest,
                            ))
                        }
                        iced::keyboard::Key::Named(iced::keyboard::key::Named::Enter) => {
                            Some(Message::SendActiveRequest)
                        }
                        _ => None,
                    }
                } else if key == iced::keyboard::Key::Named(iced::keyboard::key::Named::Enter) {
                    Some(Message::WsSendFromKeyboard)
                } else if key == iced::keyboard::Key::Named(iced::keyboard::key::Named::Escape) {
                    Some(Message::EscapePressed)
                } else {
                    None
                }
            } else {
                None
            }
        });

        let device_poll_subscription = self.device_poll_subscription();

        let menu_subscription = from_recipe(MenuEventRecipe);

        let window_opened = iced::window::open_events().map(Message::WindowOpened);

        let http_stream_subscriptions: Vec<Subscription<Message>> = self
            .http_stream_receivers
            .iter()
            .map(|(tab_index, (stream_id, receiver))| {
                from_recipe(HttpStreamRecipe {
                    receiver: receiver.clone(),
                    tab_index: *tab_index,
                    stream_id: *stream_id,
                })
            })
            .collect();

        let mut subs = vec![
            ws_subscription,
            keyboard_subscription,
            device_poll_subscription,
            menu_subscription,
            window_opened,
        ];
        subs.extend(http_stream_subscriptions);
        Subscription::batch(subs)
    }

    fn device_poll_subscription(&self) -> Subscription<Message> {
        let mut subscriptions = Vec::new();

        for (index, tab) in self.request_tabs.iter().enumerate() {
            if let crate::data::auth::Auth::OAuth2(config) = &tab.auth {
                if config.auto_polling
                    && !config.device_code.is_empty()
                    && !config.token_url.is_empty()
                {
                    let interval = config.device_code_interval.unwrap_or(5);
                    subscriptions.push(from_recipe(DevicePollRecipe {
                        tab_index: index,
                        device_code: config.device_code.clone(),
                        client_id: config.client_id.clone(),
                        client_secret: config.client_secret.clone(),
                        token_url: config.token_url.clone(),
                        interval_secs: interval,
                        http_client: self.http_client.clone(),
                    }));
                }
            }
        }

        if subscriptions.is_empty() {
            Subscription::none()
        } else {
            Subscription::batch(subscriptions)
        }
    }

    fn theme(&self) -> iced::Theme {
        if self.dark_mode {
            iced::Theme::Dark
        } else {
            iced::Theme::Light
        }
    }

    fn send_ws_message(websocket_view: &mut super::views::websocket_view::WebSocketView) {
        if let Some(sender) = &websocket_view.ws_sender {
            let input = websocket_view.input.clone();
            if !input.is_empty() && matches!(websocket_view.status, WsStatus::Connected) {
                let bytes = input.len() as u64;
                let _ = sender.send(&input);
                websocket_view.stats.messages_sent += 1;
                websocket_view.stats.bytes_sent += bytes;
                websocket_view.last_sent_message = input.clone();
                websocket_view.add_message(crate::protocols::websocket::WsMessage::outgoing(input));
                websocket_view.input.clear();
            }
        }
    }

    fn create_toolbar(&self) -> (Element<'_, Message>, Element<'_, Message>) {
        let add_tab_button = button(lucide::plus().size(16)).on_press(Message::AddRequestTab);
        let close_tab_button = if self.request_tabs.len() > 1 {
            button(lucide::x().size(16))
                .on_press(Message::CloseRequestTab(self.active_request_tab_index))
        } else {
            button(lucide::x().size(16))
        };

        let history_button = button(row![lucide::history().size(14), text(" History")].spacing(4))
            .on_press(Message::ToggleHistory);

        let collections_button =
            button(row![lucide::folder().size(14), text(" Collections")].spacing(4))
                .on_press(Message::ToggleCollections);

        let theme_button = if self.dark_mode {
            button(row![lucide::sun().size(14), text(" Light")].spacing(4))
                .on_press(Message::ToggleTheme)
        } else {
            button(row![lucide::moon().size(14), text(" Dark")].spacing(4))
                .on_press(Message::ToggleTheme)
        };

        let protocol_selector = pick_list(
            &Protocol::ALL[..],
            Some(self.active_protocol),
            Message::SelectProtocol,
        );

        let env_selector = pick_list(
            &self.environments[..],
            self.active_environment.clone(),
            |env| Message::SelectEnvironment(env.id),
        )
        .placeholder("No Environment");

        let mut env_controls = row![
            theme_button,
            protocol_selector,
            env_selector,
            button(row![lucide::settings().size(14), text(" Manage Environments")].spacing(4))
                .on_press(Message::SwitchView(View::EnvironmentManager))
        ]
        .spacing(10);

        if self.active_environment.is_some() {
            let chevron = if self.show_env_info {
                lucide::chevron_down().size(12)
            } else {
                lucide::chevron_right().size(12)
            };
            env_controls = env_controls.push(
                button(row![chevron, text(" Help").size(12)].spacing(4))
                    .on_press(Message::ToggleEnvInfo),
            );
        }

        let toolbar = row![
            add_tab_button,
            close_tab_button,
            text("").width(Length::Fixed(4.0)),
            history_button,
            collections_button,
            env_controls
        ]
        .spacing(10)
        .padding(10)
        .align_y(Alignment::Center);

        let env_help_section: Element<Message> = if let Some(active_env) = &self.active_environment
        {
            if self.show_env_info {
                let variables_text = if active_env.variables.is_empty() {
                    "This environment has no variables.".to_string()
                } else {
                    let keys: Vec<_> = active_env
                        .variables
                        .iter()
                        .map(|(k, _)| k.as_str())
                        .collect();
                    format!("Available: {}", keys.join(", "))
                };
                column![
                    text("Use {{variable}} in URL, Headers, or Body.").size(12),
                    text(variables_text).size(12)
                ]
                .spacing(5)
                .into()
            } else {
                column![].into()
            }
        } else {
            column![].into()
        };

        (toolbar.into(), env_help_section)
    }

    fn view(&self) -> Element<'_, Message> {
        match self.current_view {
            View::Main => {
                let mut tabs = Tabs::new(Message::SelectRequestTab);

                for (index, request_tab) in self.request_tabs.iter().enumerate() {
                    let tab_label = if request_tab.url_input.is_empty() {
                        TabLabel::Text(format!("New Request {}", index + 1))
                    } else {
                        let url = request_tab.url_input.chars().take(25).collect::<String>();
                        let truncated_url = if request_tab.url_input.len() > 25 {
                            format!("{url}...")
                        } else {
                            url
                        };
                        TabLabel::Text(format!("{} {}", request_tab.method, truncated_url))
                    };

                    let tab_content = if index == self.active_request_tab_index {
                        request_tab
                            .view()
                            .map(move |msg| Message::HttpRequestViewMsg(index, msg))
                    } else {
                        container(text(""))
                            .width(Length::Fill)
                            .height(Length::Fill)
                            .into()
                    };

                    tabs = tabs.push(index, tab_label, tab_content);
                }

                let tabs_widget = tabs
                    .set_active_tab(&self.active_request_tab_index)
                    .width(Length::Fill)
                    .height(Length::Fill);

                let (toolbar, env_help_section) = self.create_toolbar();

                let main_content = match self.active_protocol {
                    Protocol::Http => {
                        column![toolbar, env_help_section, tabs_widget,]
                    }
                    Protocol::WebSocket => {
                        column![
                            toolbar,
                            env_help_section,
                            self.websocket_view.view().map(Message::WebSocketMsg),
                        ]
                    }
                    Protocol::GraphQL => {
                        column![
                            toolbar,
                            env_help_section,
                            self.graphql_view.view().map(Message::GraphQLMsg),
                        ]
                    }
                    Protocol::MockServer => {
                        column![
                            toolbar,
                            env_help_section,
                            self.mock_server_view.view().map(Message::MockServerMsg),
                        ]
                    }
                };

                let toast_overlay = self
                    .toast_manager
                    .view(&self.theme())
                    .map(|()| Message::NoOp);

                let content: Element<'_, Message> = {
                    let history_panel_opt = if self.show_history {
                        Some(
                            container(self.history_view.view().map(Message::HistoryMsg))
                                .width(Length::FillPortion(1))
                                .height(Length::Fill),
                        )
                    } else {
                        None
                    };

                    let collections_panel_opt = if self.show_collections {
                        Some(
                            container(self.collection_view.view().map(Message::CollectionMsg))
                                .width(Length::FillPortion(1))
                                .height(Length::Fill),
                        )
                    } else {
                        None
                    };

                    let has_right = history_panel_opt.is_some() || collections_panel_opt.is_some();

                    let base_content: Element<'_, Message> = if has_right {
                        let mut row = row![main_content.width(Length::FillPortion(2))];
                        if let Some(p) = history_panel_opt {
                            row = row.push(rule::vertical(1)).push(p);
                        }
                        if let Some(p) = collections_panel_opt {
                            row = row.push(rule::vertical(1)).push(p);
                        }
                        container(row)
                            .width(Length::Fill)
                            .height(Length::Fill)
                            .into()
                    } else {
                        container(main_content)
                            .width(Length::Fill)
                            .height(Length::Fill)
                            .into()
                    };

                    if self.show_collection_runner {
                        if let Some(runner) = &self.collection_runner_state {
                            let runner_view = runner.view().map(Message::CollectionRunnerMsg);
                            let overlay = container(runner_view)
                                .width(Length::Fill)
                                .height(Length::Fill)
                                .padding(20);
                            stack![base_content, overlay].into()
                        } else {
                            base_content
                        }
                    } else {
                        base_content
                    }
                };

                stack![content, toast_overlay].into()
            }
            View::EnvironmentManager => self.env_manager_view.view().map(Message::EnvManagerMsg),
            View::CookieManager => self
                .cookie_manager_view
                .view()
                .map(Message::CookieManagerMsg),
        }
    }
}
