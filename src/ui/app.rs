use crate::data::auth::Auth;
use crate::persistence::database::{self, Environment};
use crate::protocols::websocket::{WsEvent, WsSender};
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
use reqwest;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;

use super::views::http_request_view::{self, HttpRequestView};
use crate::http_client::client;

use iced::futures::stream::BoxStream;
use iced::futures::{self, StreamExt as _};
use iced_futures::subscription::{from_recipe, EventStream, Recipe};

struct WsRecipe {
    receiver: Arc<Mutex<Option<mpsc::UnboundedReceiver<WsEvent>>>>,
}

impl Recipe for WsRecipe {
    type Output = Message;

    fn hash(&self, state: &mut iced_futures::subscription::Hasher) {
        use std::hash::Hash;
        std::any::TypeId::of::<WsRecipe>().hash(state);
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Protocol {
    Http,
    WebSocket,
}

impl std::fmt::Display for Protocol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Protocol::Http => write!(f, "HTTP"),
            Protocol::WebSocket => write!(f, "WebSocket"),
        }
    }
}

impl Protocol {
    pub const ALL: [Protocol; 2] = [Protocol::Http, Protocol::WebSocket];
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    Main,
    EnvironmentManager,
}

pub fn main() -> iced::Result {
    iced::application(AstraNovaApp::new, AstraNovaApp::update, AstraNovaApp::view)
        .title("AstraNova Client")
        .subscription(AstraNovaApp::subscription)
        .font(iced_fonts::LUCIDE_FONT_BYTES)
        .run()
}

struct AstraNovaApp {
    request_tabs: Vec<HttpRequestView>,
    active_request_tab_index: usize,
    http_client: reqwest::Client,
    db_conn: rusqlite::Connection,
    environments: Vec<Environment>,
    active_environment: Option<Environment>,
    env_manager_view: EnvironmentManagerView,
    history_view: HistoryView,
    collection_view: CollectionView,
    websocket_view: WebSocketView,
    active_protocol: Protocol,
    current_view: View,
    show_history: bool,
    show_collections: bool,
    show_env_info: bool,
    ws_sender: Option<WsSender>,
    ws_receiver: Option<Arc<Mutex<Option<mpsc::UnboundedReceiver<WsEvent>>>>>,
    ws_shutdown: Option<mpsc::UnboundedSender<()>>,
    ws_write_handle: Option<Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>>,
    ws_read_handle: Option<Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>>,
    toast_manager: ToastManager,
}

#[derive(Debug)]
pub enum Message {
    HttpRequestViewMsg(usize, http_request_view::Message),
    AddRequestTab,
    CloseRequestTab(usize),
    CloseActiveRequestTab,
    NoOp,
    SelectRequestTab(usize),
    EnvManagerMsg(environment_manager::Message),
    EnvFileLoaded(Option<Vec<(String, String)>>),
    SelectEnvironment(i32),
    SwitchView(View),
    HistoryMsg(history_view::Message),
    ToggleHistory,
    CollectionMsg(collection_view::Message),
    ToggleCollections,
    ToggleEnvInfo,
    WebSocketMsg(websocket_view::Message),
    WsEvent(crate::protocols::websocket::WsEvent),
    WsConnected(
        WsSender,
        Arc<Mutex<Option<mpsc::UnboundedReceiver<WsEvent>>>>,
        Option<mpsc::UnboundedSender<()>>,
        Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
        Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
    ),
    SelectProtocol(Protocol),
    OAuth2StartAuth(usize),
    OAuth2AuthComplete(usize, Result<String, String>),
    OAuth2TokenReceived(
        usize,
        Result<crate::data::oauth2::OAuth2TokenResponse, String>,
    ),
    OAuth2RefreshToken(usize),
    OAuth2StartDeviceAuth(usize),
    OAuth2DeviceAuthReceived(
        usize,
        Result<crate::data::oauth2::DeviceAuthorizationResponse, String>,
    ),
    OAuth2DeviceTokenPoll(
        usize,
        Result<crate::data::oauth2::DeviceTokenResponse, String>,
    ),
}

impl Clone for Message {
    fn clone(&self) -> Self {
        match self {
            Self::HttpRequestViewMsg(i, m) => Self::HttpRequestViewMsg(*i, m.clone()),
            Self::AddRequestTab => Self::AddRequestTab,
            Self::CloseRequestTab(i) => Self::CloseRequestTab(*i),
            Self::CloseActiveRequestTab => Self::CloseActiveRequestTab,
            Self::NoOp => Self::NoOp,
            Self::SelectRequestTab(i) => Self::SelectRequestTab(*i),
            Self::EnvManagerMsg(m) => Self::EnvManagerMsg(m.clone()),
            Self::EnvFileLoaded(v) => Self::EnvFileLoaded(v.clone()),
            Self::SelectEnvironment(i) => Self::SelectEnvironment(*i),
            Self::SwitchView(v) => Self::SwitchView(*v),
            Self::HistoryMsg(m) => Self::HistoryMsg(m.clone()),
            Self::ToggleHistory => Self::ToggleHistory,
            Self::CollectionMsg(m) => Self::CollectionMsg(m.clone()),
            Self::ToggleCollections => Self::ToggleCollections,
            Self::ToggleEnvInfo => Self::ToggleEnvInfo,
            Self::WebSocketMsg(m) => Self::WebSocketMsg(m.clone()),
            Self::WsEvent(e) => Self::WsEvent(e.clone()),
            Self::WsConnected(s, r, st, wh, rh) => Self::WsConnected(
                s.clone(),
                r.clone(),
                st.clone(),
                Arc::clone(wh),
                Arc::clone(rh),
            ),
            Self::SelectProtocol(p) => Self::SelectProtocol(*p),
            Self::OAuth2StartAuth(i) => Self::OAuth2StartAuth(*i),
            Self::OAuth2AuthComplete(i, r) => Self::OAuth2AuthComplete(*i, r.clone()),
            Self::OAuth2TokenReceived(i, r) => Self::OAuth2TokenReceived(*i, r.clone()),
            Self::OAuth2RefreshToken(i) => Self::OAuth2RefreshToken(*i),
            Self::OAuth2StartDeviceAuth(i) => Self::OAuth2StartDeviceAuth(*i),
            Self::OAuth2DeviceAuthReceived(i, r) => Self::OAuth2DeviceAuthReceived(*i, r.clone()),
            Self::OAuth2DeviceTokenPoll(i, r) => Self::OAuth2DeviceTokenPoll(*i, r.clone()),
        }
    }
}

impl AstraNovaApp {
    fn new() -> (Self, Task<Message>) {
        let (db_conn, environments) = match database::init() {
            Ok(conn) => {
                let envs = database::get_environments(&conn).unwrap_or_default();
                (conn, envs)
            }
            Err(e) => {
                log::error!("Failed to initialize database: {}", e);
                let conn = rusqlite::Connection::open_in_memory()
                    .expect("In-memory DB should always work");
                let _ = conn.execute(
                    "CREATE TABLE IF NOT EXISTS environments (
                        id INTEGER PRIMARY KEY,
                        name TEXT NOT NULL UNIQUE,
                        variables TEXT NOT NULL
                    )",
                    [],
                );
                let _ = conn.execute(
                    "ALTER TABLE environments ADD COLUMN default_endpoint TEXT",
                    [],
                );
                (conn, Vec::new())
            }
        };

        let history = crate::services::history_service::get_all(&db_conn, 50);
        let collections = crate::services::collection_service::get_all(&db_conn);

        let mut cv = CollectionView::new();
        cv.sync_collections(&collections);

        let app = Self {
            request_tabs: vec![HttpRequestView::default()],
            active_request_tab_index: 0,
            http_client: reqwest::Client::new(),
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
            toast_manager: ToastManager::new(),
        };
        (app, Task::none())
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        self.toast_manager.clean_expired();
        match message {
            Message::HttpRequestViewMsg(index, msg) => {
                if let Some(view) = self.request_tabs.get_mut(index) {
                    match msg {
                        http_request_view::Message::SendRequest => {
                            let mut temp_view = view.clone();

                            if let Some(env) = &self.active_environment {
                                temp_view.apply_environment(env);
                            }

                            let request = temp_view.build_request();

                            view.pending_request_data = serde_json::to_string(&request).ok();
                            view.update(http_request_view::Message::SetLoading);

                            let http_client = if request.config.proxy_url.is_some()
                                || !request.config.verify_ssl
                            {
                                match client::build_client(&request.config) {
                                    Ok(c) => c,
                                    Err(e) => {
                                        log::error!("Failed to build custom client: {}", e);
                                        self.http_client.clone()
                                    }
                                }
                            } else {
                                self.http_client.clone()
                            };

                            return Task::perform(
                                async move { client::send_request(&http_client, request).await },
                                move |result| {
                                    Message::HttpRequestViewMsg(
                                        index,
                                        http_request_view::Message::ResponseReceived(result),
                                    )
                                },
                            );
                        }
                        http_request_view::Message::ResponseReceived(ref result) => {
                            match result {
                                Ok(response) => {
                                    let request_data = view.pending_request_data.take();
                                    let response_data = serde_json::to_string(response).ok();
                                    let _ = crate::services::history_service::save_raw(
                                        &self.db_conn,
                                        &response.method,
                                        &response.url,
                                        Some(response.status),
                                        Some(response.duration.as_millis() as u64),
                                        request_data.as_deref(),
                                        response_data.as_deref(),
                                    );
                                    crate::services::history_service::trim(
                                        &self.db_conn,
                                        crate::persistence::database::DEFAULT_HISTORY_LIMIT,
                                    );
                                    self.history_view.entries =
                                        crate::services::history_service::get_all(
                                            &self.db_conn,
                                            50,
                                        );

                                    if response.status >= 400 {
                                        self.toast_manager.warning(format!(
                                            "{} {}",
                                            response.status, response.url
                                        ));
                                    } else {
                                        self.toast_manager.success(format!(
                                            "{} {} - {}ms",
                                            response.status,
                                            response.url,
                                            response.duration.as_millis()
                                        ));
                                    }
                                }
                                Err(e) => {
                                    self.toast_manager.error(format!("Request failed: {}", e));
                                }
                            }
                            view.update(msg);
                        }
                        http_request_view::Message::MultipartBrowseFile(entry_id) => {
                            let tab_index = index;
                            return Task::perform(
                                async {
                                    let file = rfd::AsyncFileDialog::new().pick_file().await;
                                    file.map(|f| f.path().to_string_lossy().to_string())
                                },
                                move |path| {
                                    Message::HttpRequestViewMsg(
                                        tab_index,
                                        http_request_view::Message::MultipartFilePicked(
                                            entry_id, path,
                                        ),
                                    )
                                },
                            );
                        }
                        http_request_view::Message::OAuth2StartAuth => {
                            return Task::perform(async {}, move |_| {
                                Message::OAuth2StartAuth(index)
                            });
                        }
                        http_request_view::Message::OAuth2RefreshToken => {
                            return Task::perform(async {}, move |_| {
                                Message::OAuth2RefreshToken(index)
                            });
                        }
                        http_request_view::Message::OAuth2StartDeviceAuth => {
                            return Task::perform(async {}, move |_| {
                                Message::OAuth2StartDeviceAuth(index)
                            });
                        }
                        _ => view.update(msg),
                    }
                }
            }
            Message::AddRequestTab => {
                let mut new_view = HttpRequestView::default();
                if let Some(env) = &self.active_environment {
                    if let Some(url) = &env.default_endpoint {
                        if !url.is_empty() {
                            new_view.url_input = url.clone();
                        }
                    }
                }
                self.request_tabs.push(new_view);
                self.active_request_tab_index = self.request_tabs.len() - 1;
            }
            Message::CloseRequestTab(index) => {
                if self.request_tabs.len() > 1 {
                    self.request_tabs.remove(index);
                    if self.active_request_tab_index >= self.request_tabs.len() {
                        self.active_request_tab_index = self.request_tabs.len() - 1;
                    }
                }
            }
            Message::CloseActiveRequestTab => {
                if self.request_tabs.len() > 1 {
                    let index = self.active_request_tab_index;
                    self.request_tabs.remove(index);
                    if self.active_request_tab_index >= self.request_tabs.len() {
                        self.active_request_tab_index = self.request_tabs.len() - 1;
                    }
                }
            }
            Message::NoOp => {}
            Message::SelectRequestTab(index) => {
                self.active_request_tab_index = index;
            }
            Message::EnvManagerMsg(msg) => {
                self.env_manager_view.update(msg.clone());
                match msg {
                    environment_manager::Message::CreateEnvironment => {
                        let name = self.env_manager_view.new_environment_name.clone();
                        match crate::services::environment_service::create_and_refresh(
                            &self.db_conn,
                            &name,
                        ) {
                            Ok(environments) => {
                                let new_env = environments.last().cloned();
                                self.environments = environments;
                                self.env_manager_view.environments = self.environments.clone();
                                self.env_manager_view.new_environment_name = String::new();
                                if let Some(env) = new_env {
                                    self.env_manager_view.selected_environment = Some(env);
                                }
                            }
                            Err(e) => log::error!("Error creating environment: {}", e),
                        }
                    }
                    environment_manager::Message::SaveEnvironment => {
                        if let Some(env) = &self.env_manager_view.selected_environment {
                            match crate::services::environment_service::save_and_refresh(
                                &self.db_conn,
                                env,
                            ) {
                                Ok(environments) => {
                                    self.environments = environments;
                                    self.env_manager_view.environments = self.environments.clone();
                                    if let Some(selected_env) =
                                        &self.env_manager_view.selected_environment
                                    {
                                        self.env_manager_view.selected_environment = self
                                            .environments
                                            .iter()
                                            .find(|e| e.id == selected_env.id)
                                            .cloned();
                                    }
                                }
                                Err(e) => log::error!("Error saving environment: {}", e),
                            }
                        }
                    }
                    environment_manager::Message::ConfirmDeleteEnvironment(_env_id) => {
                        if let Some(env) = &self.env_manager_view.selected_environment {
                            match crate::services::environment_service::delete_and_refresh(
                                &self.db_conn,
                                env.id,
                            ) {
                                Ok(environments) => {
                                    self.environments = environments;
                                    self.env_manager_view.environments = self.environments.clone();
                                }
                                Err(e) => log::error!("Error deleting environment: {}", e),
                            }
                        }
                    }
                    environment_manager::Message::LoadEnvFile => {
                        return Task::perform(
                            async {
                                let file = rfd::AsyncFileDialog::new().pick_file().await;
                                if let Some(file_handle) = file {
                                    let data = file_handle.read().await;
                                    let mut vars = Vec::new();
                                    if let Ok(content) = std::str::from_utf8(&data) {
                                        for line in content.lines() {
                                            let trimmed_line = line.trim();
                                            if trimmed_line.starts_with('#')
                                                || trimmed_line.is_empty()
                                            {
                                                continue;
                                            }
                                            if let Some((key, value)) = trimmed_line.split_once('=')
                                            {
                                                vars.push((
                                                    key.trim().to_string(),
                                                    value.trim().to_string(),
                                                ));
                                            }
                                        }
                                    }
                                    Some(vars)
                                } else {
                                    None
                                }
                            },
                            Message::EnvFileLoaded,
                        );
                    }
                    environment_manager::Message::Close => {
                        self.current_view = View::Main;
                    }
                    _ => (),
                }
            }
            Message::EnvFileLoaded(vars) => {
                if let Some(vars) = vars {
                    self.env_manager_view
                        .update(environment_manager::Message::UpdateVariables(vars));
                }
            }
            Message::SelectEnvironment(id) => {
                self.active_environment = self.environments.iter().find(|e| e.id == id).cloned();
            }
            Message::SwitchView(view) => {
                self.current_view = view;
            }
            Message::ToggleHistory => {
                self.show_history = !self.show_history;
            }
            Message::ToggleCollections => {
                self.show_collections = !self.show_collections;
                if self.show_collections {
                    let cols = crate::services::collection_service::get_all(&self.db_conn);
                    self.collection_view.sync_collections(&cols);
                }
            }
            Message::ToggleEnvInfo => {
                self.show_env_info = !self.show_env_info;
            }
            Message::CollectionMsg(msg) => {
                match msg.clone() {
                    collection_view::Message::NewCollectionNameChanged(name) => {
                        self.collection_view.new_collection_name = name;
                    }
                    collection_view::Message::CreateCollection => {
                        let name = self.collection_view.new_collection_name.clone();
                        if !name.is_empty() {
                            match crate::services::collection_service::create_and_refresh(
                                &self.db_conn,
                                &name,
                            ) {
                                Ok(cols) => {
                                    self.collection_view.sync_collections(&cols);
                                    self.collection_view.new_collection_name.clear();
                                }
                                Err(e) => log::error!("Error creating collection: {}", e),
                            }
                        }
                    }
                    collection_view::Message::SelectCollection(idx) => {
                        self.collection_view.panel_state =
                            collection_view::PanelState::CollectionDetail(idx);
                        if let Some(col) = self.collection_view.collections.get(idx) {
                            let col_id = col.id;
                            let folders = crate::services::collection_service::get_folders(
                                &self.db_conn,
                                col_id,
                            );
                            self.collection_view.sync_folders(&folders);
                            let reqs = crate::services::collection_service::get_requests(
                                &self.db_conn,
                                col_id,
                                None,
                            );
                            self.collection_view.sync_requests(&reqs);
                        }
                    }
                    collection_view::Message::SelectFolder(folder_id) => {
                        if let collection_view::PanelState::CollectionDetail(col_idx) =
                            self.collection_view.panel_state
                        {
                            self.collection_view.panel_state =
                                collection_view::PanelState::FolderDetail(col_idx, folder_id);
                            if let Some(col) = self.collection_view.collections.get(col_idx) {
                                let reqs = crate::services::collection_service::get_requests(
                                    &self.db_conn,
                                    col.id,
                                    Some(folder_id),
                                );
                                self.collection_view.sync_requests(&reqs);
                            }
                        }
                    }
                    collection_view::Message::Close => {
                        self.collection_view.panel_state = collection_view::PanelState::Collections;
                    }
                    collection_view::Message::NewFolderNameChanged(_col_id, name) => {
                        self.collection_view.new_folder_name = name;
                    }
                    collection_view::Message::CreateFolder(col_id) => {
                        let name = self.collection_view.new_folder_name.clone();
                        if !name.is_empty() {
                            match crate::services::collection_service::create_folder_and_refresh(
                                &self.db_conn,
                                col_id,
                                &name,
                            ) {
                                Ok(folders) => {
                                    self.collection_view.sync_folders(&folders);
                                    self.collection_view.new_folder_name.clear();
                                }
                                Err(e) => log::error!("Error creating folder: {}", e),
                            }
                        }
                    }
                    collection_view::Message::DeleteCollection(_idx) => {
                        // Local state cleanup is handled by collection_view.update()
                    }
                    collection_view::Message::ConfirmDeleteCollection(idx) => {
                        if let Some(col) = self.collection_view.collections.get(idx) {
                            match crate::services::collection_service::delete_and_refresh(
                                &self.db_conn,
                                col.id,
                            ) {
                                Ok(cols) => self.collection_view.sync_collections(&cols),
                                Err(e) => log::error!("Error deleting collection: {}", e),
                            }
                        }
                    }
                    collection_view::Message::DeleteFolder(_folder_id) => {
                        // Local state cleanup is handled by collection_view.update()
                    }
                    collection_view::Message::ConfirmDeleteFolder(folder_id) => {
                        if let collection_view::PanelState::CollectionDetail(col_idx) =
                            self.collection_view.panel_state
                        {
                            if let Some(col) = self.collection_view.collections.get(col_idx) {
                                match crate::services::collection_service::delete_folder_and_refresh(
                                    &self.db_conn,
                                    col.id,
                                    folder_id,
                                ) {
                                    Ok(folders) => self.collection_view.sync_folders(&folders),
                                    Err(e) => log::error!("Error deleting folder: {}", e),
                                }
                            }
                        }
                    }
                    collection_view::Message::ImportCollection => {
                        return Task::perform(
                            async move {
                                let file = rfd::AsyncFileDialog::new()
                                    .add_filter("Postman Collection", &["json"])
                                    .pick_file()
                                    .await;
                                if let Some(file_handle) = file {
                                    let data = file_handle.read().await;
                                    if let Ok(content) = std::str::from_utf8(&data) {
                                        return Some(content.to_string());
                                    }
                                }
                                None
                            },
                            |result| {
                                Message::CollectionMsg(
                                    collection_view::Message::ImportCollectionData(result),
                                )
                            },
                        );
                    }
                    collection_view::Message::ImportCollectionData(Some(json)) => {
                        match crate::import::postman::parse_postman_collection(&json) {
                            Ok(imported) => {
                                match crate::services::collection_service::create_and_refresh(
                                    &self.db_conn,
                                    &imported.name,
                                ) {
                                    Ok(cols) => {
                                        if let Some(new_col) = cols.last() {
                                            for folder in &imported.folders {
                                                match crate::services::collection_service::create_folder(
                                                    &self.db_conn,
                                                    new_col.id,
                                                    &folder.name,
                                                ) {
                                                    Ok(created_folder) => {
                                                        for req in &folder.requests {
                                                            let _ = crate::services::collection_service::save_request(
                                                                &self.db_conn,
                                                                new_col.id,
                                                                Some(created_folder.id),
                                                                &req.name,
                                                                &req.method,
                                                                &req.url,
                                                                &req.headers,
                                                                req.body.as_deref(),
                                                                "text",
                                                                "none",
                                                                None,
                                                                &req.params,
                                                                None,
                                                            );
                                                        }
                                                    }
                                                    Err(e) => log::error!("Error creating folder: {}", e),
                                                }
                                            }
                                            for req in &imported.requests {
                                                let _ = crate::services::collection_service::save_request(
                                                    &self.db_conn,
                                                    new_col.id,
                                                    None,
                                                    &req.name,
                                                    &req.method,
                                                    &req.url,
                                                    &req.headers,
                                                    req.body.as_deref(),
                                                    "text",
                                                    "none",
                                                    None,
                                                    &req.params,
                                                    None,
                                                );
                                            }
                                            let cols = crate::services::collection_service::get_all(
                                                &self.db_conn,
                                            );
                                            self.collection_view.sync_collections(&cols);
                                        }
                                    }
                                    Err(e) => log::error!("Error creating collection: {}", e),
                                }
                            }
                            Err(e) => log::error!("Error parsing Postman collection: {}", e),
                        }
                    }
                    collection_view::Message::ImportCollectionData(None) => {}
                    collection_view::Message::ExportCollection(idx) => {
                        if let Some(col) = self.collection_view.collections.get(idx) {
                            let folders = crate::services::collection_service::get_folders(
                                &self.db_conn,
                                col.id,
                            );
                            let requests = crate::services::collection_service::get_requests(
                                &self.db_conn,
                                col.id,
                                None,
                            );
                            match crate::export::postman::export_collection(
                                col, &folders, &requests,
                            ) {
                                Ok(json) => {
                                    let col_name = col.name.clone();
                                    return Task::perform(
                                        async move {
                                            let file = rfd::AsyncFileDialog::new()
                                                .add_filter("Postman Collection", &["json"])
                                                .set_file_name(&format!("{}.json", col_name))
                                                .save_file()
                                                .await;
                                            if let Some(file_handle) = file {
                                                let path = file_handle.path().to_path_buf();
                                                let _ =
                                                    tokio::fs::write(&path, json.as_bytes()).await;
                                            }
                                            None::<()>
                                        },
                                        |_: Option<_>| {
                                            Message::CollectionMsg(
                                                collection_view::Message::ExportCollectionData(
                                                    String::new(),
                                                ),
                                            )
                                        },
                                    );
                                }
                                Err(e) => log::error!("Error exporting collection: {}", e),
                            }
                        }
                    }
                    collection_view::Message::ExportCollectionData(_) => {}
                    collection_view::Message::ConfirmRenameCollection => {
                        if let Some(idx) = self.collection_view.renaming_collection {
                            let new_name = self.collection_view.rename_collection_value.clone();
                            if let Some(col) = self.collection_view.collections.get(idx) {
                                match crate::services::collection_service::rename(
                                    &self.db_conn,
                                    col,
                                    &new_name,
                                ) {
                                    Ok(()) => {
                                        let cols = crate::services::collection_service::get_all(
                                            &self.db_conn,
                                        );
                                        self.collection_view.sync_collections(&cols);
                                    }
                                    Err(e) => log::error!("Error renaming collection: {}", e),
                                }
                            }
                        }
                    }
                    collection_view::Message::ConfirmRenameFolder => {
                        if let Some(folder_id) = self.collection_view.renaming_folder {
                            let new_name = self.collection_view.rename_folder_value.clone();
                            match crate::services::collection_service::rename_folder(
                                &self.db_conn,
                                folder_id,
                                &new_name,
                            ) {
                                Ok(()) => {
                                    if let collection_view::PanelState::CollectionDetail(col_idx) =
                                        self.collection_view.panel_state
                                    {
                                        if let Some(col) =
                                            self.collection_view.collections.get(col_idx)
                                        {
                                            let folders =
                                                crate::services::collection_service::get_folders(
                                                    &self.db_conn,
                                                    col.id,
                                                );
                                            self.collection_view.sync_folders(&folders);
                                        }
                                    }
                                }
                                Err(e) => log::error!("Error renaming folder: {}", e),
                            }
                        }
                    }
                    collection_view::Message::ConfirmRenameRequest => {
                        if let Some(req_id) = self.collection_view.renaming_request {
                            let new_name = self.collection_view.rename_request_value.clone();
                            match crate::services::collection_service::rename_request(
                                &self.db_conn,
                                req_id,
                                &new_name,
                            ) {
                                Ok(()) => {
                                    if let collection_view::PanelState::CollectionDetail(col_idx) =
                                        self.collection_view.panel_state
                                    {
                                        if let Some(col) =
                                            self.collection_view.collections.get(col_idx)
                                        {
                                            let reqs =
                                                crate::services::collection_service::get_requests(
                                                    &self.db_conn,
                                                    col.id,
                                                    None,
                                                );
                                            self.collection_view.sync_requests(&reqs);
                                        }
                                    } else if let collection_view::PanelState::FolderDetail(
                                        col_idx,
                                        folder_id,
                                    ) = self.collection_view.panel_state
                                    {
                                        if let Some(col) =
                                            self.collection_view.collections.get(col_idx)
                                        {
                                            let reqs =
                                                crate::services::collection_service::get_requests(
                                                    &self.db_conn,
                                                    col.id,
                                                    Some(folder_id),
                                                );
                                            self.collection_view.sync_requests(&reqs);
                                        }
                                    }
                                }
                                Err(e) => log::error!("Error renaming request: {}", e),
                            }
                        }
                    }
                    collection_view::Message::DeleteRequest(_req_id) => {
                        // Local state cleanup is handled by collection_view.update()
                    }
                    collection_view::Message::ConfirmDeleteRequest(req_id) => {
                        if let collection_view::PanelState::CollectionDetail(col_idx) =
                            self.collection_view.panel_state
                        {
                            if let Some(col) = self.collection_view.collections.get(col_idx) {
                                match crate::services::collection_service::delete_request_and_refresh(
                                    &self.db_conn,
                                    col.id,
                                    None,
                                    req_id,
                                ) {
                                    Ok(reqs) => self.collection_view.sync_requests(&reqs),
                                    Err(e) => log::error!("Error deleting request: {}", e),
                                }
                            }
                        } else if let collection_view::PanelState::FolderDetail(
                            col_idx,
                            folder_id,
                        ) = self.collection_view.panel_state
                        {
                            if let Some(col) = self.collection_view.collections.get(col_idx) {
                                match crate::services::collection_service::delete_request_and_refresh(
                                    &self.db_conn,
                                    col.id,
                                    Some(folder_id),
                                    req_id,
                                ) {
                                    Ok(reqs) => self.collection_view.sync_requests(&reqs),
                                    Err(e) => log::error!("Error deleting request: {}", e),
                                }
                            }
                        }
                    }
                    collection_view::Message::LoadRequest(req_id) => {
                        self.load_collection_request(req_id);
                    }
                    collection_view::Message::SaveCurrentRequest => {
                        self.save_current_to_collection();
                    }
                    _ => {}
                }
                self.collection_view.update(msg);
            }
            Message::HistoryMsg(msg) => match msg.clone() {
                history_view::Message::ClearHistory => {
                    crate::services::history_service::clear(&self.db_conn);
                    self.history_view.update(msg);
                }
                history_view::Message::ResendEntry(entry_id) => {
                    if let Some(entry) =
                        crate::services::history_service::get_by_id(&self.db_conn, entry_id)
                    {
                        if let Some(new_view) =
                            crate::services::request_restoration::build_view_from_history(&entry)
                        {
                            self.request_tabs.push(new_view);
                            self.active_request_tab_index = self.request_tabs.len() - 1;
                        }
                    }
                    self.history_view.update(msg);
                }
                history_view::Message::SearchChanged(_) => {
                    self.history_view.update(msg);
                }
                history_view::Message::FilterMethod(_) => {
                    self.history_view.update(msg);
                }
                history_view::Message::ExportHistory => {
                    self.history_view.update(msg);
                }
            },
            Message::SelectProtocol(protocol) => {
                self.active_protocol = protocol;
            }
            Message::WsEvent(event) => match event {
                crate::protocols::websocket::WsEvent::Connected => {
                    self.websocket_view.status = crate::protocols::websocket::WsStatus::Connected;
                }
                crate::protocols::websocket::WsEvent::Message(msg) => {
                    self.websocket_view.messages.push(msg);
                }
                crate::protocols::websocket::WsEvent::Disconnected(reason) => {
                    self.websocket_view.status =
                        crate::protocols::websocket::WsStatus::Disconnected;
                    self.ws_sender = None;
                    log::info!("WebSocket disconnected: {}", reason);
                }
                crate::protocols::websocket::WsEvent::Error(e) => {
                    self.websocket_view.status =
                        crate::protocols::websocket::WsStatus::Error(e.clone());
                    self.ws_sender = None;
                    log::error!("WebSocket error: {}", e);
                }
            },
            Message::WebSocketMsg(msg) => match msg {
                websocket_view::Message::Connect => {
                    let url = self.websocket_view.url.clone();
                    let headers = self.websocket_view.headers.clone();
                    self.websocket_view.status = crate::protocols::websocket::WsStatus::Connecting;

                    return Task::perform(
                        async move {
                            let request = crate::protocols::websocket::WsRequest { url, headers };
                            crate::protocols::websocket::connect_ws(&request).await
                        },
                        |result| match result {
                            Ok(conn) => Message::WsConnected(
                                conn.sender,
                                Arc::new(Mutex::new(Some(conn.receiver))),
                                conn.shutdown_tx,
                                Arc::new(Mutex::new(Some(conn.write_handle))),
                                Arc::new(Mutex::new(Some(conn.read_handle))),
                            ),
                            Err(e) => {
                                Message::WebSocketMsg(websocket_view::Message::Disconnected(e))
                            }
                        },
                    );
                }
                websocket_view::Message::Disconnect => {
                    if let Some(shutdown_tx) = self.ws_shutdown.take() {
                        let _ = shutdown_tx.send(());
                    }
                    if let Some(handle_arc) = self.ws_write_handle.take() {
                        if let Ok(mut guard) = handle_arc.lock() {
                            if let Some(handle) = guard.take() {
                                handle.abort();
                            }
                        }
                    }
                    if let Some(handle_arc) = self.ws_read_handle.take() {
                        if let Ok(mut guard) = handle_arc.lock() {
                            if let Some(handle) = guard.take() {
                                handle.abort();
                            }
                        }
                    }
                    self.ws_sender = None;
                    self.ws_receiver = None;
                    self.websocket_view.status =
                        crate::protocols::websocket::WsStatus::Disconnected;
                }
                websocket_view::Message::Disconnected(reason) => {
                    if reason == "cleared" {
                        self.websocket_view.messages.clear();
                    } else {
                        self.ws_sender = None;
                        self.ws_receiver = None;

                        if self.websocket_view.auto_reconnect
                            && self.websocket_view.current_retries < self.websocket_view.max_retries
                        {
                            self.websocket_view.current_retries += 1;
                            self.websocket_view.status =
                                crate::protocols::websocket::WsStatus::Connecting;

                            let url = self.websocket_view.url.clone();
                            let headers = self.websocket_view.headers.clone();
                            let delay = self.websocket_view.reconnect_delay_ms;

                            log::info!(
                                "Auto-reconnect attempt {}/{} after {}ms",
                                self.websocket_view.current_retries,
                                self.websocket_view.max_retries,
                                delay
                            );

                            return Task::perform(
                                async move {
                                    tokio::time::sleep(tokio::time::Duration::from_millis(delay))
                                        .await;
                                    let request =
                                        crate::protocols::websocket::WsRequest { url, headers };
                                    crate::protocols::websocket::connect_ws(&request).await
                                },
                                |result| match result {
                                    Ok(conn) => Message::WsConnected(
                                        conn.sender,
                                        Arc::new(Mutex::new(Some(conn.receiver))),
                                        conn.shutdown_tx,
                                        Arc::new(Mutex::new(Some(conn.write_handle))),
                                        Arc::new(Mutex::new(Some(conn.read_handle))),
                                    ),
                                    Err(e) => Message::WebSocketMsg(
                                        websocket_view::Message::Disconnected(e),
                                    ),
                                },
                            );
                        } else {
                            self.websocket_view.status =
                                crate::protocols::websocket::WsStatus::Disconnected;
                            self.websocket_view.current_retries = 0;
                        }
                    }
                }
                websocket_view::Message::ToggleHeaders => {
                    self.websocket_view.show_headers = !self.websocket_view.show_headers;
                }
                websocket_view::Message::ToggleAutoReconnect => {
                    self.websocket_view.auto_reconnect = !self.websocket_view.auto_reconnect;
                    if !self.websocket_view.auto_reconnect {
                        self.websocket_view.current_retries = 0;
                    }
                }
                websocket_view::Message::ReconnectDelayChanged(delay) => {
                    if let Ok(ms) = delay.parse::<u64>() {
                        self.websocket_view.reconnect_delay_ms = ms;
                    }
                }
                websocket_view::Message::MaxRetriesChanged(retries) => {
                    if let Ok(n) = retries.parse::<u32>() {
                        self.websocket_view.max_retries = n;
                    }
                }
                websocket_view::Message::UrlChanged(url) => {
                    self.websocket_view.url = url;
                }
                websocket_view::Message::HeaderKeyChanged(key) => {
                    self.websocket_view.header_key = key;
                }
                websocket_view::Message::HeaderValueChanged(val) => {
                    self.websocket_view.header_value = val;
                }
                websocket_view::Message::AddHeader => {
                    let key = self.websocket_view.header_key.clone();
                    let val = self.websocket_view.header_value.clone();
                    if !key.is_empty() {
                        self.websocket_view.headers.push((key, val));
                        self.websocket_view.header_key.clear();
                        self.websocket_view.header_value.clear();
                    }
                }
                websocket_view::Message::RemoveHeader(idx) => {
                    if idx < self.websocket_view.headers.len() {
                        self.websocket_view.headers.remove(idx);
                    }
                }
                websocket_view::Message::InputChanged(input) => {
                    self.websocket_view.input = input;
                }
                websocket_view::Message::SendMessage(text) if !text.is_empty() => {
                    if let Some(sender) = &self.ws_sender {
                        if sender.send(&text).is_ok() {
                            self.websocket_view.messages.push(
                                crate::protocols::websocket::WsMessage::outgoing(text.clone()),
                            );
                            self.websocket_view.input.clear();
                        }
                    }
                }
                _ => {}
            },
            Message::WsConnected(sender, receiver_arc, shutdown_tx, write_handle, read_handle) => {
                self.ws_sender = Some(sender);
                self.ws_receiver = Some(receiver_arc);
                self.ws_shutdown = shutdown_tx;
                self.ws_write_handle = Some(write_handle);
                self.ws_read_handle = Some(read_handle);
                self.websocket_view.status = crate::protocols::websocket::WsStatus::Connected;
                self.websocket_view.current_retries = 0;
            }
            Message::OAuth2AuthComplete(index, result) => {
                if let Some(view) = self.request_tabs.get_mut(index) {
                    if let Auth::OAuth2(config) = &mut view.auth {
                        match result {
                            Ok(code) => {
                                let token_url = config.token_url.clone();
                                let client_id = config.client_id.clone();
                                let client_secret = config.client_secret.clone();
                                let redirect_uri = config.redirect_uri.clone();
                                let pkce_verifier = config.access_token.clone();
                                let tab_index = index;

                                return Task::perform(
                                    async move {
                                        crate::data::oauth2::exchange_code(
                                            &token_url,
                                            &code,
                                            &client_id,
                                            &client_secret,
                                            &redirect_uri,
                                            Some(&pkce_verifier),
                                        )
                                        .await
                                    },
                                    move |result| Message::OAuth2TokenReceived(tab_index, result),
                                );
                            }
                            Err(e) => {
                                log::error!("OAuth2 authorization failed: {}", e);
                            }
                        }
                    }
                }
            }
            Message::OAuth2TokenReceived(index, result) => {
                if let Some(view) = self.request_tabs.get_mut(index) {
                    if let Auth::OAuth2(config) = &mut view.auth {
                        match result {
                            Ok(token_response) => {
                                config.access_token = token_response.access_token;
                                if let Some(refresh) = token_response.refresh_token {
                                    config.refresh_token = refresh;
                                }
                                log::info!("OAuth2 token received successfully");
                            }
                            Err(e) => {
                                log::error!("OAuth2 token exchange failed: {}", e);
                            }
                        }
                    }
                }
            }
            Message::OAuth2StartAuth(index) => {
                if let Some(view) = self.request_tabs.get(index) {
                    if let Auth::OAuth2(config) = &view.auth {
                        let pkce = if config.pkce_enabled {
                            Some(crate::data::oauth2::PKCEChallenge::generate())
                        } else {
                            None
                        };

                        let state = crate::data::oauth2::generate_state();
                        let auth_url = crate::data::oauth2::build_authorization_url(
                            &config.auth_url,
                            &config.client_id,
                            &config.redirect_uri,
                            &config.scopes,
                            pkce.as_ref(),
                            &state,
                        );

                        let verifier = pkce.map(|p| p.verifier);

                        return Task::perform(
                            async move {
                                let _ = open::that(&auth_url);
                                verifier.ok_or_else(|| "No PKCE verifier".to_string())
                            },
                            move |result| Message::OAuth2AuthComplete(index, result),
                        );
                    }
                }
            }
            Message::OAuth2RefreshToken(index) => {
                if let Some(view) = self.request_tabs.get(index) {
                    if let Auth::OAuth2(config) = &view.auth {
                        if !config.device_code.is_empty() {
                            let token_url = config.token_url.clone();
                            let device_code = config.device_code.clone();
                            let client_id = config.client_id.clone();
                            let client_secret = config.client_secret.clone();
                            let tab_index = index;

                            return Task::perform(
                                async move {
                                    crate::data::oauth2::poll_device_token(
                                        &token_url,
                                        &device_code,
                                        &client_id,
                                        &client_secret,
                                    )
                                    .await
                                },
                                move |result| Message::OAuth2DeviceTokenPoll(tab_index, result),
                            );
                        } else if config.refresh_token.is_empty() {
                            log::warn!("No refresh token available");
                        } else {
                            let token_url = config.token_url.clone();
                            let refresh_token = config.refresh_token.clone();
                            let client_id = config.client_id.clone();
                            let client_secret = config.client_secret.clone();
                            let tab_index = index;

                            return Task::perform(
                                async move {
                                    crate::data::oauth2::refresh_token(
                                        &token_url,
                                        &refresh_token,
                                        &client_id,
                                        &client_secret,
                                    )
                                    .await
                                },
                                move |result| Message::OAuth2TokenReceived(tab_index, result),
                            );
                        }
                    }
                }
            }
            Message::OAuth2StartDeviceAuth(index) => {
                if let Some(view) = self.request_tabs.get(index) {
                    if let Auth::OAuth2(config) = &view.auth {
                        if config.device_auth_url.is_empty() {
                            log::warn!("No device authorization URL configured");
                        } else {
                            let device_auth_url = config.device_auth_url.clone();
                            let client_id = config.client_id.clone();
                            let scopes = config.scopes.clone();
                            let tab_index = index;

                            return Task::perform(
                                async move {
                                    crate::data::oauth2::device_authorization(
                                        &device_auth_url,
                                        &client_id,
                                        &scopes,
                                    )
                                    .await
                                },
                                move |result| Message::OAuth2DeviceAuthReceived(tab_index, result),
                            );
                        }
                    }
                }
            }
            Message::OAuth2DeviceAuthReceived(index, result) => {
                if let Some(view) = self.request_tabs.get_mut(index) {
                    if let Auth::OAuth2(config) = &mut view.auth {
                        match result {
                            Ok(device_auth) => {
                                config.device_code = device_auth.device_code;
                                config.user_code = device_auth.user_code;
                                config.verification_uri = device_auth.verification_uri;
                                config.device_code_expires_in = Some(device_auth.expires_in);
                                config.device_code_interval = device_auth.interval;

                                let verification_url = config.verification_uri.clone();
                                let user_code = config.user_code.clone();

                                log::info!(
                                    "Device authorization received. User code: {}",
                                    user_code
                                );

                                let _ = open::that(&verification_url);
                            }
                            Err(e) => {
                                log::error!("Device authorization failed: {}", e);
                            }
                        }
                    }
                }
            }
            Message::OAuth2DeviceTokenPoll(index, result) => {
                if let Some(view) = self.request_tabs.get_mut(index) {
                    if let Auth::OAuth2(config) = &mut view.auth {
                        match result {
                            Ok(device_token) => {
                                if let Some(access_token) = device_token.access_token {
                                    config.access_token = access_token;
                                    if let Some(refresh) = device_token.refresh_token {
                                        config.refresh_token = refresh;
                                    }
                                    config.device_code.clear();
                                    config.user_code.clear();
                                    config.verification_uri.clear();
                                    log::info!("Device token received successfully");
                                } else if let Some(error) = device_token.error {
                                    if error == "authorization_pending" {
                                        log::info!("Authorization pending, polling again...");
                                    } else if error == "slow_down" {
                                        log::warn!("Slow down detected, increasing interval");
                                    } else {
                                        log::error!("Device token error: {}", error);
                                        config.device_code.clear();
                                        config.user_code.clear();
                                        config.verification_uri.clear();
                                    }
                                }
                            }
                            Err(e) => {
                                log::error!("Device token poll failed: {}", e);
                            }
                        }
                    }
                }
            }
        }
        Task::none()
    }

    fn subscription(&self) -> Subscription<Message> {
        let ws_subscription = if let Some(receiver_arc) = &self.ws_receiver {
            from_recipe(WsRecipe {
                receiver: receiver_arc.clone(),
            })
        } else {
            Subscription::none()
        };

        let keyboard_subscription = iced::keyboard::listen().map(|event| match event {
            iced::keyboard::Event::KeyPressed { key, modifiers, .. } => {
                if modifiers.control() {
                    match key {
                        iced::keyboard::Key::Character(ref c) if c.as_ref() == "n" => {
                            Message::AddRequestTab
                        }
                        iced::keyboard::Key::Character(ref c) if c.as_ref() == "w" => {
                            Message::CloseActiveRequestTab
                        }
                        iced::keyboard::Key::Character(ref c) if c.as_ref() == "1" => {
                            Message::SelectRequestTab(0)
                        }
                        iced::keyboard::Key::Character(ref c) if c.as_ref() == "2" => {
                            Message::SelectRequestTab(1)
                        }
                        iced::keyboard::Key::Character(ref c) if c.as_ref() == "3" => {
                            Message::SelectRequestTab(2)
                        }
                        iced::keyboard::Key::Character(ref c) if c.as_ref() == "4" => {
                            Message::SelectRequestTab(3)
                        }
                        iced::keyboard::Key::Character(ref c) if c.as_ref() == "5" => {
                            Message::SelectRequestTab(4)
                        }
                        _ => Message::NoOp,
                    }
                } else {
                    Message::NoOp
                }
            }
            _ => Message::NoOp,
        });

        Subscription::batch(vec![ws_subscription, keyboard_subscription])
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
                            format!("{}...", url)
                        } else {
                            url
                        };
                        TabLabel::Text(format!("{} {}", request_tab.method, truncated_url))
                    };

                    tabs = tabs.push(
                        index,
                        tab_label,
                        request_tab
                            .view()
                            .map(move |msg| Message::HttpRequestViewMsg(index, msg)),
                    );
                }

                let tabs_widget = tabs
                    .set_active_tab(&self.active_request_tab_index)
                    .width(Length::Fill)
                    .height(Length::Fill);

                let add_tab_button =
                    button(lucide::plus().size(16)).on_press(Message::AddRequestTab);
                let close_tab_button = if self.request_tabs.len() > 1 {
                    button(lucide::x().size(16))
                        .on_press(Message::CloseRequestTab(self.active_request_tab_index))
                } else {
                    button(lucide::x().size(16))
                };

                let history_button =
                    button(row![lucide::history().size(14), text(" History")].spacing(4))
                        .on_press(Message::ToggleHistory);

                let collections_button =
                    button(row![lucide::folder().size(14), text(" Collections")].spacing(4))
                        .on_press(Message::ToggleCollections);

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
                    history_button,
                    collections_button,
                    protocol_selector,
                    env_selector,
                    button(
                        row![lucide::settings().size(14), text(" Manage Environments")].spacing(4)
                    )
                    .on_press(Message::SwitchView(View::EnvironmentManager))
                ]
                .spacing(10);

                if let Some(active_env) = &self.active_environment {
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

                let env_help_section: Element<Message> =
                    if let Some(active_env) = &self.active_environment {
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

                let main_content = match self.active_protocol {
                    Protocol::Http => {
                        column![
                            row![add_tab_button, close_tab_button, env_controls,]
                                .spacing(10)
                                .padding(10)
                                .align_y(Alignment::Center),
                            env_help_section,
                            tabs_widget,
                        ]
                    }
                    Protocol::WebSocket => {
                        column![
                            row![add_tab_button, close_tab_button, env_controls,]
                                .spacing(10)
                                .padding(10)
                                .align_y(Alignment::Center),
                            env_help_section,
                            self.websocket_view.view().map(Message::WebSocketMsg),
                        ]
                    }
                };

                let toast_overlay = self.toast_manager.view().map(|_| Message::NoOp);

                let content: Element<'_, Message> = if self.show_history {
                    let history_panel =
                        container(self.history_view.view().map(Message::HistoryMsg))
                            .width(Length::FillPortion(1))
                            .height(Length::Fill);

                    if self.show_collections {
                        let collections_panel =
                            container(self.collection_view.view().map(Message::CollectionMsg))
                                .width(Length::FillPortion(1))
                                .height(Length::Fill);

                        let content = row![
                            main_content.width(Length::FillPortion(2)),
                            rule::vertical(1),
                            history_panel.width(Length::FillPortion(1)),
                            rule::vertical(1),
                            collections_panel.width(Length::FillPortion(1)),
                        ];

                        container(content)
                            .width(Length::Fill)
                            .height(Length::Fill)
                            .into()
                    } else {
                        let content = row![
                            main_content.width(Length::FillPortion(3)),
                            rule::vertical(1),
                            history_panel,
                        ];

                        container(content)
                            .width(Length::Fill)
                            .height(Length::Fill)
                            .into()
                    }
                } else if self.show_collections {
                    let collections_panel =
                        container(self.collection_view.view().map(Message::CollectionMsg))
                            .width(Length::FillPortion(1))
                            .height(Length::Fill);

                    let content = row![
                        main_content.width(Length::FillPortion(3)),
                        rule::vertical(1),
                        collections_panel,
                    ];

                    container(content)
                        .width(Length::Fill)
                        .height(Length::Fill)
                        .into()
                } else {
                    container(main_content)
                        .width(Length::Fill)
                        .height(Length::Fill)
                        .into()
                };

                stack![content, toast_overlay].into()
            }
            View::EnvironmentManager => self.env_manager_view.view().map(Message::EnvManagerMsg),
        }
    }

    fn load_collection_request(&mut self, req_id: i32) {
        let conn = &self.db_conn;
        let all_reqs = match &self.collection_view.panel_state {
            collection_view::PanelState::CollectionDetail(idx) => {
                if let Some(col) = self.collection_view.collections.get(*idx) {
                    crate::services::collection_service::get_requests(conn, col.id, None)
                } else {
                    return;
                }
            }
            collection_view::PanelState::FolderDetail(_col_idx, _folder_id) => {
                self.collection_view.requests.clone()
            }
            _ => return,
        };

        let req = match all_reqs.iter().find(|r| r.id == req_id) {
            Some(r) => r.clone(),
            None => return,
        };

        let new_view =
            crate::services::request_restoration::build_view_from_collection_request(&req);
        self.request_tabs.push(new_view);
        self.active_request_tab_index = self.request_tabs.len() - 1;
    }

    fn save_current_to_collection(&mut self) {
        if let Some(view) = self.request_tabs.get(self.active_request_tab_index) {
            let col_id = match self.collection_view.selected_collection_id {
                Some(id) => id,
                None => {
                    if let Some(col) = self.collection_view.collections.first() {
                        col.id
                    } else {
                        return;
                    }
                }
            };

            let request = view.build_request();
            let auth_type = match &view.auth {
                crate::data::auth::Auth::BearerToken(_) => "bearer",
                crate::data::auth::Auth::Basic { .. } => "basic",
                crate::data::auth::Auth::ApiKey { .. } => "api_key",
                crate::data::auth::Auth::Digest { .. } => "digest",
                crate::data::auth::Auth::OAuth2(_) => "oauth2",
                crate::data::auth::Auth::None => "none",
            };
            let auth_data = match &view.auth {
                crate::data::auth::Auth::None => None,
                auth => serde_json::to_string(auth).ok(),
            };

            let params: Vec<(String, String)> = view
                .params_editor
                .entries
                .iter()
                .filter(|p| !p.key.is_empty())
                .map(|p| (p.key.clone(), p.value.clone()))
                .collect();

            let body_type = match view.body_type {
                http_request_view::BodyType::Multipart => "multipart",
                _ => "text",
            };

            let name = if request.url.len() > 40 {
                format!("{} {}", request.method, &request.url[..40])
            } else {
                format!("{} {}", request.method, request.url)
            };

            let _ = crate::services::collection_service::save_request(
                &self.db_conn,
                col_id,
                None,
                &name,
                &request.method,
                &request.url,
                &request.headers,
                request.body.as_deref(),
                body_type,
                auth_type,
                auth_data.as_deref(),
                &params,
                None,
            );

            let reqs =
                crate::services::collection_service::get_requests(&self.db_conn, col_id, None);
            self.collection_view.sync_requests(&reqs);
        }
    }
}
