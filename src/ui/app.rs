use crate::persistence::database::{self, Environment};
use crate::ui::views::environment_manager::{self, EnvironmentManagerView};
use crate::ui::views::history_view::{self, HistoryView};
use iced::{
    widget::{button, column, container, pick_list, row, text},
    Alignment, Element, Length, Task,
};
use iced_aw::{TabLabel, Tabs};
use reqwest;

use super::views::http_request_view::{self, HttpRequestView};
use crate::http_client::client;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    Main,
    EnvironmentManager,
}

pub fn main() -> iced::Result {
    iced::application("AstraNova Client", AstraNovaApp::update, AstraNovaApp::view)
        .run_with(AstraNovaApp::new)
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
    current_view: View,
    show_history: bool,
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
    LoadHistory,
    HistoryLoaded(Vec<database::RequestHistoryEntry>),
    ApplyHistoryEntry(database::RequestHistoryEntry),
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
            current_view: View::Main,
            show_history: false,
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
            Message::LoadHistory => {
                let history = database::get_request_history(&self.db_conn, 50)
                    .unwrap_or_default();
                self.history_view.entries = history;
            }
            Message::HistoryLoaded(entries) => {
                self.history_view.entries = entries;
            }
            Message::HistoryMsg(msg) => {
                if let Some(entry) = self.history_view.update(msg) {
                    self.request_tabs.push(HttpRequestView::default());
                    self.active_request_tab_index = self.request_tabs.len() - 1;
                    if let Some(view) = self.request_tabs.last_mut() {
                        view.url_input = entry.url;
                        view.method = Box::leak(entry.method.into_boxed_str());
                    }
                }
            }
            Message::ApplyHistoryEntry(entry) => {
                self.request_tabs.push(HttpRequestView::default());
                self.active_request_tab_index = self.request_tabs.len() - 1;
                if let Some(view) = self.request_tabs.last_mut() {
                    view.url_input = entry.url;
                    view.method = Box::leak(entry.method.into_boxed_str());
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

                let env_selector = pick_list(
                    &self.environments[..],
                    self.active_environment.clone(),
                    |env| Message::SelectEnvironment(env.id),
                )
                .placeholder("No Environment");

                let mut env_controls = row![
                    history_button,
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

                let main_content = column![
                    row![add_tab_button, close_tab_button, env_controls,]
                        .spacing(10)
                        .padding(10)
                        .align_y(Alignment::Center),
                    tabs_widget,
                ];

                if self.show_history {
                    let history_panel = container(
                        self.history_view.view().map(Message::HistoryMsg)
                    )
                    .width(Length::FillPortion(1))
                    .height(Length::Fill);

                    row![
                        main_content.width(Length::FillPortion(3)),
                        iced::widget::Rule::vertical(1),
                        history_panel,
                    ]
                    .into()
                } else {
                    main_content.into()
                }
            }
            View::EnvironmentManager => self.env_manager_view.view().map(Message::EnvManagerMsg),
        }
    }
}
