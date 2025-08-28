use iced::{widget::{column, row, text_input, button, text, scrollable, image::{self, Handle}, container, pick_list}, Element};

static HTTP_METHODS: [&str; 5] = ["GET", "POST", "PUT", "PATCH", "DELETE"];

#[derive(Debug, Clone)]
pub enum Message {
    UrlInputChanged(String),
    MethodSelected(String),
    HeadersInputChanged(String),
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
    pub headers_input: String,
    pub body_input: String,
    request_status: RequestStatus,
    pub status_code: Option<u16>,
    pub content_type: Option<String>,
}

impl HttpRequestView {
    pub fn new() -> Self {
        Self {
            url_input: "https://jsonplaceholder.typicode.com/todos/1".to_string(),
            method: "GET".to_string(),
            headers_input: "".to_string(),
            body_input: "".to_string(),
            request_status: RequestStatus::Idle,
            status_code: None,
            content_type: None,
        }
    }

    pub fn update(&mut self, message: Message) {
        match message {
            Message::UrlInputChanged(url) => {
                self.url_input = url;
            }
            Message::MethodSelected(method) => {
                self.method = method;
            }
            Message::HeadersInputChanged(headers) => {
                self.headers_input = headers;
            }
            Message::BodyInputChanged(body) => {
                self.body_input = body;
            }
            Message::SendRequest => {
                self.request_status = RequestStatus::Loading;
                self.status_code = None;
                self.content_type = None;
            }
            Message::ResponseReceived(result) => {
                match result {
                    Ok(response) => {
                        self.status_code = Some(response.status);
                        let content_type = response.headers.iter()
                            .find(|(k, _)| k.eq_ignore_ascii_case("content-type"))
                            .map(|(_, v)| v.clone())
                            .unwrap_or_else(|| "unknown".to_string());
                        self.content_type = Some(content_type.clone());

                        let formatted_body = if content_type.contains("application/json") {
                            match serde_json::from_str::<serde_json::Value>(&response.body) {
                                Ok(json_value) => {
                                    match serde_json::to_string_pretty(&json_value) {
                                        Ok(pretty_json) => pretty_json,
                                        Err(_) => response.body.clone(), 
                                    }
                                },
                                Err(_) => response.body.clone(),
                            }
                        } else {
                            response.body.clone()
                        };

                        self.request_status = RequestStatus::Success(format!(
                            r#"Headers: {headers:#?}

Body: {body}"#,
                            headers = response.headers,
                            body = formatted_body,
                        ));
                    }
                    Err(e) => {
                        self.request_status = RequestStatus::Error(format!("Error: {}", e));
                        self.status_code = None;
                        self.content_type = None;
                    }
                }
            }
            Message::CopyResponse => {
                if let RequestStatus::Success(response_text) = &self.request_status {
                    let mut clipboard = arboard::Clipboard::new().unwrap();
                    clipboard.set_text(response_text.clone()).unwrap();
                } else if let RequestStatus::Error(error_message) = &self.request_status {
                    let mut clipboard = arboard::Clipboard::new().unwrap();
                    clipboard.set_text(error_message.clone()).unwrap();
                }
            }
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let response_content_widget = match &self.request_status {
            RequestStatus::Idle => {
                let content = container(
                    text("Enter URL and send request.")
                        .width(iced::Length::Fill)
                        .height(iced::Length::Fill)
                )
                .width(iced::Length::Fill)
                .height(iced::Length::Fill)
                .center_x()
                .center_y();
                Element::new(content)
            },
            RequestStatus::Loading => text("Loading...")
                .width(iced::Length::Fill)
                .height(iced::Length::Fill)
                .into(),
            RequestStatus::Success(response_text) => scrollable(column![text(response_text)].padding(10))
                .width(iced::Length::Fill)
                .height(iced::Length::Fill)
                .into(),
            RequestStatus::Error(error_message) => text(format!("Error: {}", error_message))
                .width(iced::Length::Fill)
                .height(iced::Length::Fill)
                .into(),
        };

        let copy_button = if matches!(self.request_status, RequestStatus::Success(_) | RequestStatus::Error(_)) {
            Element::new(button("Copy").on_press(Message::CopyResponse))
        } else {
            Element::from(column![])
        };

        let response_area = container(response_content_widget)
            .width(iced::Length::Fill)
            .height(iced::Length::Fill);

        let status_code_text = if let Some(code) = self.status_code {
            text(format!("Status: {}", code)).size(16)
        } else {
            text("Status: N/A").size(16)
        };

        let content_type_text = if let Some(ctype) = &self.content_type {
            text(format!("Content-Type: {}", ctype)).size(16)
        } else {
            text("Content-Type: N/A").size(16)
        };

        column![
            image::Image::new(Handle::from_path("assets/logo-bg.png")).width(iced::Length::Fixed(100.0)).height(iced::Length::Fixed(100.0)),
            row![
                text_input("URL", &self.url_input)
                    .on_input(Message::UrlInputChanged)
                    .padding(10)
                    .width(iced::Length::Fill),
                pick_list(
                    &HTTP_METHODS[..],
                    Some(self.method.as_str()),
                    |s| Message::MethodSelected(s.to_string()),
                ),
                button("Send").on_press(Message::SendRequest)
            ]
            .spacing(10)
            .padding(10),
            row![
                text_input("Headers (e.g., Content-Type: application/json)", &self.headers_input)
                    .on_input(Message::HeadersInputChanged)
                    .padding(10)
                    .width(iced::Length::Fill),
            ]
            .spacing(10)
            .padding(10),
            row![
                text_input("Request Body", &self.body_input)
                    .on_input(Message::BodyInputChanged)
                    .padding(10)
                    .width(iced::Length::Fill),
            ]
            .spacing(10)
            .padding(10),
            row![
                status_code_text,
                content_type_text,
            ]
            .spacing(10)
            .padding(10),
            row![
                response_area,
                copy_button,
            ]
            .spacing(10)
            .padding(10),
        ]
        .into()
    }
}
