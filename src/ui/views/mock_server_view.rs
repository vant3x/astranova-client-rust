use crate::protocols::mock_server::{MockServerConfig, MockServerLog, MockServerStatus};
use crate::ui::components::key_value_editor::{self, KeyValueEditor};
use iced::widget::rule;
use iced::widget::text_editor;

use iced::{
    widget::{button, column, container, pick_list, row, scrollable, text, text_input},
    Alignment, Color, Element, Length, Renderer, Theme,
};
use iced_fonts::lucide;

const HTTP_METHODS: &[&str] = &["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS"];

#[derive(Debug, Clone)]
pub enum Message {
    CreateServer(String),
    NewServerNameChanged(String),
    SelectServer(Option<i32>),
    DeleteServer(i32),
    StartServer(i32),
    StopServer(i32),
    ToggleAddServer,
    AddEndpoint(i32),
    EditEndpoint(i32),
    EndpointMethodSelected(String),
    EndpointPathChanged(String),
    EndpointStatusChanged(String),
    EndpointBodyAction(text_editor::Action),
    EndpointDelayChanged(String),
    EndpointHeadersEditor(key_value_editor::Message),
    SaveEndpoint,
    CancelEndpointEdit,
    DeleteEndpoint(i32, i32),
    EndpointSearchChanged(String),
    ClearLogs,
}

#[derive(Debug, Clone)]
pub struct EndpointEditState {
    pub mock_server_id: i32,
    pub endpoint_id: Option<i32>,
    pub method: String,
    pub path: String,
    pub status: String,
    pub body: text_editor::Content,
    pub delay_ms: String,
    pub headers: KeyValueEditor,
}

#[derive(Debug, Clone, Default)]
pub struct MockServerView {
    pub servers: Vec<MockServerConfig>,
    pub selected_server_id: Option<i32>,
    pub statuses: std::collections::HashMap<i32, MockServerStatus>,
    pub logs: Vec<MockServerLog>,
    pub new_server_name: String,
    pub endpoint_edit: Option<EndpointEditState>,
    pub endpoint_search: String,
    pub show_add_server: bool,
}

impl MockServerView {
    pub fn sync_servers(&mut self, servers: &[MockServerConfig]) {
        self.servers = servers.to_vec();
    }

    pub fn view(&self) -> Element<'_, Message, Theme, Renderer> {
        let sidebar = self.render_server_sidebar();
        let content = self.render_server_content();

        row![
            container(sidebar).width(Length::Fixed(260.0)),
            rule::vertical(1),
            container(content).width(Length::Fill),
        ]
        .spacing(0)
        .into()
    }

    fn render_server_sidebar(&self) -> Element<'_, Message, Theme, Renderer> {
        let header = row![
            text("Mock Servers").size(16),
            button(lucide::plus().size(14)).on_press(Message::ToggleAddServer),
        ]
        .spacing(8)
        .align_y(Alignment::Center);

        let mut server_list = column![].spacing(4);

        for server in &self.servers {
            let is_selected = self.selected_server_id == Some(server.id);
            let status = self.statuses.get(&server.id).cloned().unwrap_or_default();

            let status_icon = match &status {
                MockServerStatus::Running { .. } => lucide::play().size(12),
                MockServerStatus::Starting => lucide::loader().size(12),
                MockServerStatus::Error(_) => lucide::circle_x().size(12),
                MockServerStatus::Stopped => lucide::square().size(12),
            };

            let status_color = match &status {
                MockServerStatus::Running { .. } => Color::from_rgb(0.2, 0.7, 0.3),
                MockServerStatus::Starting => Color::from_rgb(0.8, 0.7, 0.1),
                MockServerStatus::Error(_) => Color::from_rgb(0.8, 0.2, 0.2),
                MockServerStatus::Stopped => Color::from_rgb(0.5, 0.5, 0.5),
            };

            let server_label = row![
                status_icon.color(status_color),
                text(format!("{}:{}", server.name, server.port)).size(13),
                text(format!("{} ep", server.endpoints.len()))
                    .size(11)
                    .color(Color::from_rgb(0.5, 0.5, 0.5)),
            ]
            .spacing(6)
            .align_y(Alignment::Center);

            let server_btn = if is_selected {
                button(server_label)
                    .style(iced::widget::button::primary)
                    .width(Length::Fill)
            } else {
                button(server_label).width(Length::Fill)
            }
            .on_press(Message::SelectServer(Some(server.id)));

            let delete_btn =
                button(lucide::trash().size(11)).on_press(Message::DeleteServer(server.id));

            server_list = server_list.push(
                row![server_btn, delete_btn]
                    .spacing(4)
                    .align_y(Alignment::Center),
            );
        }

        let add_section = if self.show_add_server {
            column![
                rule::horizontal(1),
                text_input("Server name", &self.new_server_name)
                    .on_input(Message::NewServerNameChanged)
                    .padding(6),
                row![
                    button(text("Create").size(12))
                        .on_press(Message::CreateServer(self.new_server_name.clone())),
                    button(text("Cancel").size(12)).on_press(Message::ToggleAddServer),
                ]
                .spacing(8),
            ]
            .spacing(8)
        } else {
            column![]
        };

        column![
            header.padding(10),
            scrollable(server_list).height(Length::Fill),
            add_section.padding(10),
        ]
        .spacing(8)
        .into()
    }

    fn render_server_content(&self) -> Element<'_, Message, Theme, Renderer> {
        let server = match self
            .selected_server_id
            .and_then(|id| self.servers.iter().find(|s| s.id == id))
        {
            Some(s) => s,
            None => {
                return container(
                    text("Select a mock server")
                        .size(14)
                        .color(Color::from_rgb(0.5, 0.5, 0.5)),
                )
                .center_x(Length::Fill)
                .center_y(Length::Fill)
                .into();
            }
        };

        let status = self.statuses.get(&server.id).cloned().unwrap_or_default();

        let status_text = match &status {
            MockServerStatus::Running { actual_port } => {
                text(format!("Running on 127.0.0.1:{}", actual_port))
                    .color(Color::from_rgb(0.2, 0.7, 0.3))
            }
            MockServerStatus::Starting => text("Starting...").color(Color::from_rgb(0.8, 0.7, 0.1)),
            MockServerStatus::Error(e) => {
                text(format!("Error: {}", e)).color(Color::from_rgb(0.8, 0.2, 0.2))
            }
            MockServerStatus::Stopped => text("Stopped").color(Color::from_rgb(0.5, 0.5, 0.5)),
        };

        let start_stop_btn = match &status {
            MockServerStatus::Running { .. } => {
                button(row![lucide::square().size(14), text(" Stop")].spacing(4))
                    .on_press(Message::StopServer(server.id))
            }
            _ => button(row![lucide::play().size(14), text(" Start")].spacing(4))
                .on_press(Message::StartServer(server.id)),
        };

        let header = row![
            text(&server.name).size(16),
            text(format!(":{}", server.port))
                .size(14)
                .color(Color::from_rgb(0.5, 0.5, 0.5)),
            status_text.size(12),
            start_stop_btn,
        ]
        .spacing(12)
        .align_y(Alignment::Center);

        let endpoints_section = self.render_endpoints_section(server);
        let logs_section = self.render_logs_section();

        column![
            header.padding(10),
            rule::horizontal(1),
            endpoints_section,
            rule::horizontal(1),
            logs_section,
        ]
        .spacing(0)
        .into()
    }

    fn render_endpoints_section(
        &self,
        server: &MockServerConfig,
    ) -> Element<'_, Message, Theme, Renderer> {
        let server = server.clone();
        let search = text_input("Search endpoints...", &self.endpoint_search)
            .on_input(Message::EndpointSearchChanged)
            .padding(6);

        let add_btn = button(row![lucide::plus().size(12), text(" Add Endpoint")].spacing(4))
            .on_press(Message::AddEndpoint(server.id));

        let header_row = row![search.width(Length::Fill), add_btn]
            .spacing(8)
            .align_y(Alignment::Center);

        let mut endpoints_list = column![].spacing(2);

        let search_lower = self.endpoint_search.to_lowercase();
        let mut has_endpoints = false;

        for ep in &server.endpoints {
            let matches_search = search_lower.is_empty()
                || ep.method.to_lowercase().contains(&search_lower)
                || ep.path.to_lowercase().contains(&search_lower);

            if !matches_search {
                continue;
            }
            has_endpoints = true;

            let method_color = match ep.method.as_str() {
                "GET" => Color::from_rgb(0.2, 0.7, 0.3),
                "POST" => Color::from_rgb(0.2, 0.5, 0.8),
                "PUT" | "PATCH" => Color::from_rgb(0.8, 0.7, 0.1),
                "DELETE" => Color::from_rgb(0.8, 0.2, 0.2),
                _ => Color::from_rgb(0.5, 0.5, 0.5),
            };

            let status_color = if ep.status >= 200 && ep.status < 300 {
                Color::from_rgb(0.2, 0.7, 0.3)
            } else if ep.status >= 400 {
                Color::from_rgb(0.8, 0.2, 0.2)
            } else {
                Color::from_rgb(0.8, 0.7, 0.1)
            };

            let body_preview = ep
                .body
                .as_deref()
                .unwrap_or("")
                .chars()
                .take(30)
                .collect::<String>();
            let method = ep.method.clone();
            let path = ep.path.clone();
            let ep_id = ep.id;
            let status = ep.status;

            let ep_row = row![
                text(method)
                    .size(12)
                    .color(method_color)
                    .width(Length::Fixed(60.0)),
                text(path).size(12).width(Length::Fill),
                text(status.to_string())
                    .size(12)
                    .color(status_color)
                    .width(Length::Fixed(40.0)),
                text(body_preview)
                    .size(11)
                    .color(Color::from_rgb(0.4, 0.4, 0.4))
                    .width(Length::FillPortion(2)),
                button(lucide::pencil().size(11)).on_press(Message::EditEndpoint(ep_id)),
                button(lucide::trash().size(11))
                    .on_press(Message::DeleteEndpoint(ep_id, server.id)),
            ]
            .spacing(8)
            .align_y(Alignment::Center);

            endpoints_list = endpoints_list.push(ep_row);
        }

        if !has_endpoints {
            endpoints_list = endpoints_list.push(
                container(
                    text("No endpoints configured")
                        .size(13)
                        .color(Color::from_rgb(0.5, 0.5, 0.5)),
                )
                .center_x(Length::Fill)
                .padding(20),
            );
        }

        let endpoint_form = if let Some(ref edit) = self.endpoint_edit {
            container(self.render_endpoint_form(edit))
        } else {
            container(column![])
        };

        if self.endpoint_edit.is_some() {
            column![header_row.padding(10), endpoint_form.height(Length::Fill),]
                .spacing(0)
                .into()
        } else {
            column![
                header_row.padding(10),
                scrollable(endpoints_list.padding(8)).height(Length::Fill),
            ]
            .spacing(0)
            .into()
        }
    }

    fn render_endpoint_form<'a>(
        &self,
        edit: &'a EndpointEditState,
    ) -> Element<'a, Message, Theme, Renderer> {
        let title = if edit.endpoint_id.is_some() {
            "Edit Endpoint"
        } else {
            "New Endpoint"
        };

        let method_selector = pick_list(HTTP_METHODS, Some(edit.method.as_str()), |s| {
            Message::EndpointMethodSelected(s.to_string())
        })
        .padding(10)
        .width(Length::Fixed(120.0));

        let path_input = text_input("/api/users", &edit.path)
            .on_input(Message::EndpointPathChanged)
            .padding(10)
            .width(Length::Fill);

        let status_input = text_input("200", &edit.status)
            .on_input(Message::EndpointStatusChanged)
            .padding(10)
            .width(Length::Fixed(100.0));

        let delay_input = text_input("0", &edit.delay_ms)
            .on_input(Message::EndpointDelayChanged)
            .padding(10)
            .width(Length::Fixed(120.0));

        let body_editor = text_editor(&edit.body)
            .on_action(Message::EndpointBodyAction)
            .padding(10)
            .height(Length::Fixed(120.0));

        let save_btn = button(row![lucide::check().size(14), text(" Save").size(13)].spacing(4))
            .on_press(Message::SaveEndpoint)
            .padding(iced::Padding::from([8, 16]));

        let cancel_btn = button(row![lucide::x().size(14), text(" Cancel").size(13)].spacing(4))
            .on_press(Message::CancelEndpointEdit)
            .padding(iced::Padding::from([8, 16]));

        container(
            column![
                container(
                    row![lucide::pencil().size(16), text(title).size(15),]
                        .spacing(8)
                        .align_y(Alignment::Center),
                )
                .padding(iced::Padding::from([8, 16])),
                scrollable(
                    column![
                        column![
                            text("Method & Path")
                                .size(12)
                                .color(Color::from_rgb(0.5, 0.5, 0.5)),
                            row![method_selector, path_input]
                                .spacing(8)
                                .align_y(Alignment::Center),
                        ]
                        .spacing(4),
                        column![
                            text("Status Code")
                                .size(12)
                                .color(Color::from_rgb(0.5, 0.5, 0.5)),
                            row![
                                status_input,
                                text("Delay").size(12).color(Color::from_rgb(0.5, 0.5, 0.5)),
                                delay_input,
                                text("ms").size(12).color(Color::from_rgb(0.5, 0.5, 0.5)),
                            ]
                            .spacing(8)
                            .align_y(Alignment::Center),
                        ]
                        .spacing(4),
                        column![
                            text("Response Body")
                                .size(12)
                                .color(Color::from_rgb(0.5, 0.5, 0.5)),
                            body_editor,
                        ]
                        .spacing(4),
                        column![
                            text("Response Headers")
                                .size(12)
                                .color(Color::from_rgb(0.5, 0.5, 0.5)),
                            edit.headers.view().map(Message::EndpointHeadersEditor),
                        ]
                        .spacing(4),
                        row![save_btn, cancel_btn]
                            .spacing(12)
                            .align_y(Alignment::Center),
                    ]
                    .spacing(12)
                    .padding(iced::Padding::from([0, 16])),
                )
                .height(Length::Fill),
            ]
            .spacing(0),
        )
        .into()
    }

    fn render_logs_section(&self) -> Element<'_, Message, Theme, Renderer> {
        let mut log_list = column![].spacing(2);

        let selected = self.selected_server_id;
        let filtered_logs: Vec<&MockServerLog> = self
            .logs
            .iter()
            .filter(|log| selected == Some(log.mock_server_id))
            .collect();

        let header = row![
            text("Request Log").size(14),
            text(format!("({})", filtered_logs.len()))
                .size(12)
                .color(Color::from_rgb(0.5, 0.5, 0.5)),
            button(row![lucide::trash().size(11), text(" Clear").size(11)].spacing(4))
                .on_press(Message::ClearLogs),
        ]
        .spacing(8)
        .align_y(Alignment::Center);

        for log in filtered_logs.iter().rev().take(50) {
            let method_color = match log.method.as_str() {
                "GET" => Color::from_rgb(0.2, 0.7, 0.3),
                "POST" => Color::from_rgb(0.2, 0.5, 0.8),
                "PUT" | "PATCH" => Color::from_rgb(0.8, 0.7, 0.1),
                "DELETE" => Color::from_rgb(0.8, 0.2, 0.2),
                _ => Color::from_rgb(0.5, 0.5, 0.5),
            };

            let status_color = if log.status >= 200 && log.status < 300 {
                Color::from_rgb(0.2, 0.7, 0.3)
            } else if log.status >= 400 {
                Color::from_rgb(0.8, 0.2, 0.2)
            } else {
                Color::from_rgb(0.8, 0.7, 0.1)
            };

            let log_row = row![
                text(&log.timestamp)
                    .size(11)
                    .color(Color::from_rgb(0.4, 0.4, 0.4))
                    .width(Length::Fixed(140.0)),
                text(&log.method)
                    .size(11)
                    .color(method_color)
                    .width(Length::Fixed(50.0)),
                text(&log.path).size(11).width(Length::Fill),
                text(log.status.to_string())
                    .size(11)
                    .color(status_color)
                    .width(Length::Fixed(35.0)),
                text(format!("{}ms", log.response_time_ms))
                    .size(11)
                    .color(Color::from_rgb(0.4, 0.4, 0.4))
                    .width(Length::Fixed(50.0)),
            ]
            .spacing(6)
            .align_y(Alignment::Center);

            log_list = log_list.push(log_row);
        }

        if filtered_logs.is_empty() {
            log_list = log_list.push(
                container(
                    text("No requests logged yet")
                        .size(12)
                        .color(Color::from_rgb(0.5, 0.5, 0.5)),
                )
                .center_x(Length::Fill)
                .padding(20),
            );
        }

        column![
            header.padding(10),
            scrollable(log_list.padding(10)).height(Length::Fixed(200.0)),
        ]
        .spacing(4)
        .into()
    }
}
