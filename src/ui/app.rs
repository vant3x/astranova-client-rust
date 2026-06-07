use crate::persistence::database::{self, Environment};
use crate::ui::views::collection_view::{self, CollectionView};
use crate::ui::views::environment_manager::{self, EnvironmentManagerView};
use crate::ui::views::history_view::{self, HistoryView};
use crate::ui::views::websocket_view::{self, WebSocketView};
use iced::{
    widget::{button, column, container, pick_list, row, rule, text},
    Alignment, Element, Length, Task,
};
use iced_aw::{TabLabel, Tabs};
use reqwest;

use super::views::http_request_view::{self, HttpRequestView};
use crate::http_client::client;

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
}

#[derive(Debug, Clone)]
pub enum Message {
    HttpRequestViewMsg(usize, http_request_view::Message),
    AddRequestTab,
    CloseRequestTab(usize),
    SelectRequestTab(usize),
    EnvManagerMsg(environment_manager::Message),
    EnvFileLoaded(Option<Vec<(String, String)>>),
    SelectEnvironment(i32),
    SwitchView(View),
    HistoryMsg(history_view::Message),
    ToggleHistory,
    CollectionMsg(collection_view::Message),
    ToggleCollections,
    WebSocketMsg(websocket_view::Message),
    SelectProtocol(Protocol),
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
                let conn = rusqlite::Connection::open_in_memory().expect("In-memory DB should always work");
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

        let history = database::get_request_history(&db_conn, 50).unwrap_or_default();
        let collections = database::get_collections(&db_conn).unwrap_or_default();

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
        };
        (app, Task::none())
    }

    fn update(&mut self, message: Message) -> Task<Message> {
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
                            if let Ok(response) = result {
                                let _ = database::save_request_history(
                                    &self.db_conn,
                                    &response.method,
                                    &response.url,
                                    Some(response.status),
                                    Some(response.duration.as_millis() as u64),
                                );
                                let history =
                                    database::get_request_history(&self.db_conn, 50)
                                        .unwrap_or_default();
                                self.history_view.entries = history;
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
                                        http_request_view::Message::MultipartFilePicked(entry_id, path),
                                    )
                                },
                            );
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
            Message::SelectRequestTab(index) => {
                self.active_request_tab_index = index;
            }
            Message::EnvManagerMsg(msg) => {
                self.env_manager_view.update(msg.clone());
                match msg {
                    environment_manager::Message::CreateEnvironment => {
                        match database::create_environment(
                            &self.db_conn,
                            &self.env_manager_view.new_environment_name,
                        ) {
                            Ok(new_env) => {
                                self.environments.push(new_env.clone());
                                self.env_manager_view.environments = self.environments.clone();
                                self.env_manager_view.new_environment_name = String::new();
                                self.env_manager_view.selected_environment = Some(new_env);
                            }
                            Err(e) => log::error!("Error creating environment: {}", e),
                        }
                    }
                    environment_manager::Message::SaveEnvironment => {
                        if let Some(env) = &self.env_manager_view.selected_environment {
                            match database::update_environment(&self.db_conn, env) {
                                Ok(_) => match database::get_environments(&self.db_conn) {
                                    Ok(environments) => {
                                        self.environments = environments;
                                        self.env_manager_view.environments =
                                            self.environments.clone();
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
                                    Err(e) => {
                                        log::error!("Error getting environments after save: {}", e)
                                    }
                                },
                                Err(e) => log::error!("Error saving environment: {}", e),
                            }
                        }
                    }
                    environment_manager::Message::DeleteEnvironment => {
                        if let Some(env) = &self.env_manager_view.selected_environment {
                            match database::delete_environment(&self.db_conn, env.id) {
                                Ok(_) => match database::get_environments(&self.db_conn) {
                                    Ok(environments) => {
                                        self.environments = environments;
                                        self.env_manager_view.environments =
                                            self.environments.clone();
                                    }
                                    Err(e) => {
                                        log::error!("Error getting environments after delete: {}", e)
                                    }
                                },
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
                    self.refresh_collections();
                }
            }
            Message::CollectionMsg(msg) => {
                match msg.clone() {
                    collection_view::Message::NewCollectionNameChanged(name) => {
                        self.collection_view.new_collection_name = name;
                    }
                    collection_view::Message::CreateCollection => {
                        let name = self.collection_view.new_collection_name.clone();
                        if !name.is_empty() {
                            match database::create_collection(&self.db_conn, &name, None) {
                                Ok(_col) => {
                                    self.refresh_collections();
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
                            match database::get_folders(&self.db_conn, col_id) {
                                Ok(folders) => self.collection_view.sync_folders(&folders),
                                Err(e) => log::error!("Error getting folders: {}", e),
                            }
                            match database::get_collection_requests(&self.db_conn, col_id, None) {
                                Ok(reqs) => self.collection_view.sync_requests(&reqs),
                                Err(e) => log::error!("Error getting requests: {}", e),
                            }
                        }
                    }
                    collection_view::Message::SelectFolder(folder_id) => {
                        if let collection_view::PanelState::CollectionDetail(col_idx) =
                            self.collection_view.panel_state
                        {
                            self.collection_view.panel_state =
                                collection_view::PanelState::FolderDetail(col_idx, folder_id);
                            if let Some(col) = self.collection_view.collections.get(col_idx) {
                                match database::get_collection_requests(
                                    &self.db_conn,
                                    col.id,
                                    Some(folder_id),
                                ) {
                                    Ok(reqs) => self.collection_view.sync_requests(&reqs),
                                    Err(e) => log::error!("Error getting folder requests: {}", e),
                                }
                            }
                        }
                    }
                    collection_view::Message::Close => {
                        self.collection_view.panel_state =
                            collection_view::PanelState::Collections;
                    }
                    collection_view::Message::NewFolderNameChanged(_col_id, name) => {
                        self.collection_view.new_folder_name = name;
                    }
                    collection_view::Message::CreateFolder(col_id) => {
                        let name = self.collection_view.new_folder_name.clone();
                        if !name.is_empty() {
                            match database::create_folder(&self.db_conn, col_id, &name, None) {
                                Ok(_) => {
                                    self.refresh_collection_folders(col_id);
                                    self.collection_view.new_folder_name.clear();
                                }
                                Err(e) => log::error!("Error creating folder: {}", e),
                            }
                        }
                    }
                    collection_view::Message::DeleteCollection(idx) => {
                        if let Some(col) = self.collection_view.collections.get(idx) {
                            let _ = database::delete_collection(&self.db_conn, col.id);
                            self.refresh_collections();
                        }
                    }
                    collection_view::Message::DeleteFolder(folder_id) => {
                        let _ = database::delete_folder(&self.db_conn, folder_id);
                        if let collection_view::PanelState::CollectionDetail(col_idx) =
                            self.collection_view.panel_state
                        {
                            if let Some(col) = self.collection_view.collections.get(col_idx) {
                                self.refresh_collection_folders(col.id);
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
            Message::HistoryMsg(msg) => {
                match &msg {
                    history_view::Message::ClearHistory => {
                        let _ = database::delete_request_history(&self.db_conn);
                        self.history_view.update(msg);
                    }
                    _ => {
                        if let Some(entry) = self.history_view.update(msg) {
                            self.request_tabs.push(HttpRequestView::default());
                            self.active_request_tab_index = self.request_tabs.len() - 1;
                            if let Some(view) = self.request_tabs.last_mut() {
                                view.url_input = entry.url;
                                view.method = Box::leak(entry.method.into_boxed_str());
                            }
                        }
                    }
                }
            }
            Message::SelectProtocol(protocol) => {
                self.active_protocol = protocol;
            }
            Message::WebSocketMsg(msg) => {
                match msg.clone() {
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
                                Ok((_sink, _stream)) => {
                                    Message::WebSocketMsg(websocket_view::Message::Connected)
                                }
                                Err(e) => {
                                    Message::WebSocketMsg(websocket_view::Message::Disconnected(e))
                                }
                            },
                        );
                    }
                    websocket_view::Message::Connected => {
                        self.websocket_view.status = crate::protocols::websocket::WsStatus::Connected;
                    }
                    websocket_view::Message::Disconnected(reason) => {
                        if reason == "cleared" {
                            self.websocket_view.messages.clear();
                        } else {
                            self.websocket_view.status =
                                crate::protocols::websocket::WsStatus::Error(reason);
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
                    websocket_view::Message::SendMessage(text) => {
                        if !text.is_empty() {
                            self.websocket_view
                                .messages
                                .push(crate::protocols::websocket::WsMessage::outgoing(text));
                            self.websocket_view.input.clear();
                        }
                    }
                    _ => {}
                }
            }
        }
        Task::none()
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

                let add_tab_button = button(text("+")).on_press(Message::AddRequestTab);
                let close_tab_button = if self.request_tabs.len() > 1 {
                    button(text("x"))
                        .on_press(Message::CloseRequestTab(self.active_request_tab_index))
                } else {
                    button(text("x"))
                };

                let history_button = button(text("History"))
                    .on_press(Message::ToggleHistory);

                let collections_button = button(text("Collections"))
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
                    button(text("Manage Environments"))
                        .on_press(Message::SwitchView(View::EnvironmentManager))
                ]
                .spacing(10);

                if let Some(active_env) = &self.active_environment {
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

                    let help_texts = column![
                        text("Use {{variable}} in URL, Headers, or Body.").size(12),
                        text(variables_text).size(12)
                    ]
                    .spacing(5);

                    env_controls = env_controls.push(help_texts);
                }

                let main_content = match self.active_protocol {
                    Protocol::Http => {
                        column![
                            row![add_tab_button, close_tab_button, env_controls,]
                                .spacing(10)
                                .padding(10)
                                .align_y(Alignment::Center),
                            tabs_widget,
                        ]
                    }
                    Protocol::WebSocket => {
                        column![
                            row![add_tab_button, close_tab_button, env_controls,]
                                .spacing(10)
                                .padding(10)
                                .align_y(Alignment::Center),
                            self.websocket_view.view().map(Message::WebSocketMsg),
                        ]
                    }
                };

                if self.show_history {
                    let history_panel = container(
                        self.history_view.view().map(Message::HistoryMsg)
                    )
                    .width(Length::FillPortion(1))
                    .height(Length::Fill);

                    if self.show_collections {
                        let collections_panel = container(
                            self.collection_view.view().map(Message::CollectionMsg)
                        )
                        .width(Length::FillPortion(1))
                        .height(Length::Fill);

                        row![
                            main_content.width(Length::FillPortion(2)),
                            rule::vertical(1),
                            history_panel.width(Length::FillPortion(1)),
                            rule::vertical(1),
                            collections_panel.width(Length::FillPortion(1)),
                        ]
                        .into()
                    } else {
                        row![
                            main_content.width(Length::FillPortion(3)),
                            rule::vertical(1),
                            history_panel,
                        ]
                        .into()
                    }
                } else if self.show_collections {
                    let collections_panel = container(
                        self.collection_view.view().map(Message::CollectionMsg)
                    )
                    .width(Length::FillPortion(1))
                    .height(Length::Fill);

                    row![
                        main_content.width(Length::FillPortion(3)),
                        rule::vertical(1),
                        collections_panel,
                    ]
                    .into()
                } else {
                    main_content.into()
                }
            }
            View::EnvironmentManager => self.env_manager_view.view().map(Message::EnvManagerMsg),
        }
    }

    fn refresh_collections(&mut self) {
        if let Ok(collections) = database::get_collections(&self.db_conn) {
            self.collection_view.sync_collections(&collections);
        }
    }

    fn refresh_collection_folders(&mut self, col_id: i32) {
        if let Ok(folders) = database::get_folders(&self.db_conn, col_id) {
            self.collection_view.sync_folders(&folders);
        }
    }

    fn load_collection_request(&mut self, req_id: i32) {
        let conn = &self.db_conn;
        let all_reqs = match &self.collection_view.panel_state {
            collection_view::PanelState::CollectionDetail(idx) => {
                if let Some(col) = self.collection_view.collections.get(*idx) {
                    database::get_collection_requests(conn, col.id, None).unwrap_or_default()
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

        let mut new_view = HttpRequestView::default();
        new_view.url_input = req.url;
        new_view.method = Box::leak(req.method.into_boxed_str());

        if let Some(body) = &req.body {
            new_view.body_input = iced::widget::text_editor::Content::with_text(body);
        }

        if req.body_type == "multipart" {
            new_view.body_type = http_request_view::BodyType::Multipart;
        }

        new_view.headers_editor.entries = req
            .headers
            .iter()
            .map(|(k, v)| crate::ui::components::key_value_editor::KeyValueEntry {
                id: 0,
                key: k.clone(),
                value: v.clone(),
            })
            .collect();

        new_view.params_editor.entries = req
            .params
            .iter()
            .map(|(k, v)| crate::ui::components::key_value_editor::KeyValueEntry {
                id: 0,
                key: k.clone(),
                value: v.clone(),
            })
            .collect();

        match req.auth_type.as_str() {
            "bearer" => {
                if let Some(token) = &req.auth_data {
                    new_view.auth = crate::data::auth::Auth::BearerToken(token.clone());
                }
            }
            "basic" => {
                if let Some(data) = &req.auth_data {
                    let parts: Vec<&str> = data.splitn(2, ':').collect();
                    if parts.len() == 2 {
                        new_view.auth = crate::data::auth::Auth::Basic {
                            user: parts[0].to_string(),
                            pass: parts[1].to_string(),
                        };
                    }
                }
            }
            _ => {}
        }

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
                crate::data::auth::Auth::None => "none",
            };
            let auth_data = match &view.auth {
                crate::data::auth::Auth::BearerToken(token) => Some(token.clone()),
                crate::data::auth::Auth::Basic { user, pass } => {
                    Some(format!("{}:{}", user, pass))
                }
                _ => None,
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

            let _ = database::save_collection_request(
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

            match database::get_collection_requests(&self.db_conn, col_id, None) {
                Ok(reqs) => self.collection_view.sync_requests(&reqs),
                Err(e) => log::error!("Error refreshing requests: {}", e),
            }
        }
    }
}
