use iced::{widget::{column, row, text_input, button, text, scrollable, image::{self, Handle}, container, pick_list, Rule}, Element, Alignment};
use iced_aw::{TabLabel, Tabs};
use std::time::Duration;

use crate::ui::components::key_value_editor::{self, KeyValueEditor};

const LOGO_BG_BYTES: &[u8] = include_bytes!("../../../assets/logo-bg.png");

static HTTP_METHODS: [&str; 5] = ["GET", "POST", "PUT", "PATCH", "DELETE"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RequestTabId {
    Body,
    Headers,
    Params,
}

#[derive(Debug, Clone)]
pub enum Message {
    UrlInputChanged(String),
    MethodSelected(String),
    RequestTabSelected(RequestTabId),
    HeadersEditorMessage(key_value_editor::Message),
    ParamsEditorMessage(key_value_editor::Message),
    BodyInputChanged(String),
    SendRequest,
    ResponseReceived(Result<crate::http_client::response::HttpResponse, String>),
    CopyResponse,
}

#[derive(Debug, Clone)]
pub enum RequestStatus {
    Idle,
    Loading,
    Success(String),
    Error(String),
}

pub struct HttpRequestView {
    pub url_input: String,
    pub method: String,
    pub body_input: String,
    pub headers_editor: KeyValueEditor,
    pub params_editor: KeyValueEditor,
    active_request_tab: RequestTabId,
    request_status: RequestStatus,
    pub status_code: Option<u16>,
    pub content_type: Option<String>,
    pub response_duration: Option<Duration>,
    pub response_size: Option<u64>,
}

impl HttpRequestView {
    pub fn new() -> Self {
        Self {
            url_input: "https://jsonplaceholder.typicode.com/todos/1".to_string(),
            method: "GET".to_string(),
            body_input: "".to_string(),
            headers_editor: KeyValueEditor::new(),
            params_editor: KeyValueEditor::new(),
            active_request_tab: RequestTabId::Body,
            request_status: RequestStatus::Idle,
            status_code: None,
            content_type: None,
            response_duration: None,
            response_size: None,
        }
    }

    pub fn update(&mut self, message: Message) {
        match message {
            Message::UrlInputChanged(url) => self.url_input = url,
            Message::MethodSelected(method) => self.method = method,
            Message::RequestTabSelected(tab_id) => self.active_request_tab = tab_id,
            Message::HeadersEditorMessage(msg) => self.headers_editor.update(msg),
            Message::ParamsEditorMessage(msg) => self.params_editor.update(msg),
            Message::BodyInputChanged(body) => self.body_input = body,
            Message::SendRequest => {
                self.request_status = RequestStatus::Loading;
                self.status_code = None;
                self.content_type = None;
                self.response_duration = None;
                self.response_size = None;
            }
            Message::ResponseReceived(result) => {
                match result {
                    Ok(response) => {
                        self.status_code = Some(response.status);
                        self.response_duration = Some(response.duration);
                        self.response_size = Some(response.size);
                        let content_type = response.headers.iter()
                            .find(|(k, _)| k.eq_ignore_ascii_case("content-type"))
                            .map(|(_, v)| v.clone())
                            .unwrap_or_else(|| "unknown".to_string());
                        self.content_type = Some(content_type.clone());

                        let formatted_body = if content_type.contains("application/json") {
                            match serde_json::from_str::<serde_json::Value>(&response.body) {
                                Ok(json_value) => serde_json::to_string_pretty(&json_value).unwrap_or_else(|_| response.body.clone()),
                                Err(_) => response.body.clone(),
                            }
                        } else {
                            response.body.clone()
                        };

                        self.request_status = RequestStatus::Success(format!(
                            r#"Headers: {headers:#?}

Body: {body}

--------------------
URL: {url}
Method: {method}"#,
                            headers = response.headers,
                            body = formatted_body,
                            url = response.url,
                            method = response.method,
                        ));
                    }
                    Err(e) => {
                        self.request_status = RequestStatus::Error(format!("Error: {}", e));
                        self.status_code = None;
                        self.content_type = None;
                        self.response_duration = None;
                        self.response_size = None;
                    }
                }
            }
            Message::CopyResponse => {
                let text_to_copy = match &self.request_status {
                    RequestStatus::Success(response_text) => Some(response_text.clone()),
                    RequestStatus::Error(error_message) => Some(error_message.clone()),
                    _ => None,
                };

                if let Some(text) = text_to_copy {
                    let mut clipboard = arboard::Clipboard::new().unwrap();
                    clipboard.set_text(text).unwrap();
                }
            }
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let request_tabs = Tabs::new(Message::RequestTabSelected)
            .set_active_tab(&self.active_request_tab)
            .push(RequestTabId::Body, TabLabel::Text("Body".to_string()), text_input("Request Body", &self.body_input).on_input(Message::BodyInputChanged).padding(10))
            .push(RequestTabId::Headers, TabLabel::Text("Headers".to_string()), self.headers_editor.view().map(Message::HeadersEditorMessage))
            .push(RequestTabId::Params, TabLabel::Text("Params".to_string()), self.params_editor.view().map(Message::ParamsEditorMessage));

        let response_area: Element<Message> = match &self.request_status {
            RequestStatus::Idle => container(text("Enter URL and send request."))
                .width(iced::Length::Fill)
                .height(iced::Length::Fill)
                .center_x()
                .center_y()
                .into(),
            RequestStatus::Loading => container(text("Loading..."))
                .width(iced::Length::Fill)
                .height(iced::Length::Fill)
                .center_x()
                .center_y()
                .into(),
            RequestStatus::Success(response_text) => container(scrollable(text(response_text)))
                .width(iced::Length::Fill)
                .height(iced::Length::Fill)
                .into(),
            RequestStatus::Error(error_message) => container(text(format!("Error: {}", error_message)))
                .width(iced::Length::Fill)
                .height(iced::Length::Fill)
                .center_x()
                .center_y()
                .into(),
        };

        let copy_button = if matches!(self.request_status, RequestStatus::Success(_) | RequestStatus::Error(_)) {
            button("Copy").on_press(Message::CopyResponse).into()
        } else {
            Element::from(column![])
        };

        let status_code_text = text(format!("Status: {}", self.status_code.map(|s| s.to_string()).unwrap_or_else(|| "N/A".to_string()))).size(16);
        let content_type_text = text(format!("Content-Type: {}", self.content_type.as_deref().unwrap_or("N/A"))).size(16);
        let duration_text = text(format!("Time: {}", self.response_duration.map(|d| format!("{}ms", d.as_millis())).unwrap_or_else(|| "N/A".to_string()))).size(16);
        let size_text = text(format!("Size: {}", self.response_size.map(|s| format!("{} B", s)).unwrap_or_else(|| "N/A".to_string()))).size(16);


        let main_column = column![
            image::Image::new(Handle::from_memory(LOGO_BG_BYTES.to_vec())).width(iced::Length::Fixed(100.0)).height(iced::Length::Fixed(100.0)),
            row![
                pick_list(&HTTP_METHODS[..], Some(self.method.as_str()), |s| Message::MethodSelected(s.to_string())),
                text_input("URL", &self.url_input).on_input(Message::UrlInputChanged).padding(10),
                button("Send").on_press(Message::SendRequest)
            ].spacing(10).padding(10),
            
            request_tabs,

            text(format!("Debug: Active Tab is {:?}", self.active_request_tab)),

            Rule::horizontal(10),

            row![
                status_code_text,
                content_type_text,
                duration_text,
                size_text,
            ].spacing(20).padding(10),

            row![
                response_area,
                copy_button,
            ].spacing(10).padding(10).height(iced::Length::Fill),
        ]
        .align_items(Alignment::Center);

        main_column.into()
    }
}
