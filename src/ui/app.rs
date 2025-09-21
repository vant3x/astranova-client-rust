use crate::persistence::database::{self, Environment};
use crate::ui::views::environment_manager::{self, EnvironmentManagerView};
use iced::{
    widget::{button, column, row, text, pick_list, container},
    Element, Length, Task,
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
    current_view: View,
}

#[derive(Debug, Clone)]
pub enum Message {
    HttpRequestViewMsg(usize, http_request_view::Message),
    AddRequestTab,
    CloseRequestTab(usize),
    SelectRequestTab(usize),
    Loaded(Result<Vec<Environment>, String>),
    EnvManagerMsg(environment_manager::Message),
    SelectEnvironment(i32),
    SwitchView(View),
}

impl AstraNovaApp {
    fn new() -> (Self, Task<Message>) {
        let db_conn = database::init().unwrap();
        let environments = database::get_environments(&db_conn).unwrap();
        let app = Self {
            request_tabs: vec![HttpRequestView::default()],
            active_request_tab_index: 0,
            http_client: reqwest::Client::new(),
            db_conn,
            environments: environments.clone(),
            active_environment: None,
            env_manager_view: EnvironmentManagerView::new(environments),
            current_view: View::Main,
        };
        (app, Task::none())
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::HttpRequestViewMsg(index, msg) => {
                if let Some(view) = self.request_tabs.get_mut(index) {
                    if let http_request_view::Message::SendRequest(mut request) = msg {
                        if let Some(env) = &self.active_environment {
                            for (key, value) in &env.variables {
                                let placeholder = format!("{{{{{}}}}}", key);
                                request.url = request.url.replace(&placeholder, value);
                                if let Some(body) = &mut request.body {
                                    *body = body.replace(&placeholder, value);
                                }
                                for (_, header_value) in &mut request.headers {
                                    *header_value = header_value.replace(&placeholder, value);
                                }
                            }
                        }

                        view.update(http_request_view::Message::SetLoading);

                        let http_client = self.http_client.clone(); // Clone the client for the async task
                        return Task::perform(
                            async move { client::send_request(&http_client, request).await },
                            move |result| {
                                Message::HttpRequestViewMsg(
                                    index,
                                    http_request_view::Message::ResponseReceived(result),
                                )
                            },
                        );
                    } else {
                        view.update(msg);
                    }
                }
            }
            Message::AddRequestTab => {
                self.request_tabs.push(HttpRequestView::default());
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
            Message::Loaded(Ok(environments)) => {
                self.environments = environments;
                self.env_manager_view = EnvironmentManagerView::new(self.environments.clone());
            }
            Message::Loaded(Err(e)) => {
                eprintln!("Error loading environments: {}", e);
            }
            Message::EnvManagerMsg(msg) => {
                self.env_manager_view.update(msg.clone());
                match msg {
                    environment_manager::Message::CreateEnvironment => {
                        match database::create_environment(&self.db_conn, &self.env_manager_view.new_environment_name) {
                            Ok(new_env) => {
                                self.environments.push(new_env.clone());
                                self.env_manager_view.environments = self.environments.clone();
                                self.env_manager_view.new_environment_name = String::new();
                                self.env_manager_view.selected_environment = Some(new_env);
                            }
                            Err(e) => eprintln!("Error creating environment: {}", e),
                        }
                    }
                    environment_manager::Message::SaveEnvironment => {
                        if let Some(env) = &self.env_manager_view.selected_environment {
                            match database::update_environment(&self.db_conn, env) {
                                Ok(_) => {
                                    match database::get_environments(&self.db_conn) {
                                        Ok(environments) => {
                                            self.environments = environments;
                                            self.env_manager_view.environments = self.environments.clone();
                                            if let Some(selected_env) = &self.env_manager_view.selected_environment {
                                                self.env_manager_view.selected_environment = self.environments.iter().find(|e| e.id == selected_env.id).cloned();
                                            }
                                        }
                                        Err(e) => eprintln!("Error getting environments after save: {}", e),
                                    }
                                }
                                Err(e) => eprintln!("Error saving environment: {}", e),
                            }
                        }
                    }
                    environment_manager::Message::DeleteEnvironment => {
                        if let Some(env) = &self.env_manager_view.selected_environment {
                            match database::delete_environment(&self.db_conn, env.id) {
                                Ok(_) => {
                                    match database::get_environments(&self.db_conn) {
                                        Ok(environments) => {
                                            self.environments = environments;
                                            self.env_manager_view.environments = self.environments.clone();
                                        }
                                        Err(e) => eprintln!("Error getting environments after delete: {}", e),
                                    }
                                }
                                Err(e) => eprintln!("Error deleting environment: {}", e),
                            }
                        }
                    }
                    environment_manager::Message::Close => {
                        self.current_view = View::Main;
                    }
                    _ => ()
                }
            }
            Message::SelectEnvironment(id) => {
                self.active_environment = self.environments.iter().find(|e| e.id == id).cloned();
            }
            Message::SwitchView(view) => {
                self.current_view = view;
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
                    button(text("x")).on_press(Message::CloseRequestTab(self.active_request_tab_index))
                } else {
                    button(text("x"))
                };

                let env_selector = pick_list(
                    self.environments.iter().map(|e| e.id).collect::<Vec<_>>(),
                    self.active_environment.as_ref().map(|e| e.id),
                    Message::SelectEnvironment,
                )
                .placeholder("No Environment");

                column![
                    row![add_tab_button, close_tab_button, env_selector, button(text("Manage Environments")).on_press(Message::SwitchView(View::EnvironmentManager))]
                        .spacing(10)
                        .padding(10),
                    tabs_widget,
                ].into()
            }
            View::EnvironmentManager => self.env_manager_view.view().map(Message::EnvManagerMsg),
        }
    }
}



