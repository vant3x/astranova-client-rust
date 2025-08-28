use iced::{widget::{column, row, text_input, button, text, scrollable, image::{self, Handle}, container}, Element};

#[derive(Debug, Clone)]
pub enum Message {
    UrlInputChanged(String),
    MethodSelected(String),
    SendRequest,
    ResponseReceived(Result<crate::http_client::response::HttpResponse, String>),
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
    request_status: RequestStatus,
}

impl HttpRequestView {
    pub fn new() -> Self {
        Self {
            url_input: "https://jsonplaceholder.typicode.com/todos/1".to_string(),
            method: "GET".to_string(),
            request_status: RequestStatus::Idle,
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
            Message::SendRequest => {
                self.request_status = RequestStatus::Loading;
            }
            Message::ResponseReceived(result) => {
                match result {
                    Ok(response) => {
                        self.request_status = RequestStatus::Success(format!(r#"Status: {}\nHeaders: {:#?}\nBody: {}"#,
                            response.status,
                            response.headers,
                            response.body,
                        ));
                    }
                    Err(e) => {
                        self.request_status = RequestStatus::Error(format!("Error: {}", e));
                    }
                }
            }
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let response_content = match &self.request_status {
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

        column![
            image::Image::new(Handle::from_path("assets/logo-bg.png")).width(iced::Length::Fixed(100.0)).height(iced::Length::Fixed(100.0)),
            row![
                text_input("URL", &self.url_input)
                    .on_input(Message::UrlInputChanged)
                    .padding(10)
                    .width(iced::Length::Fill),
                button("GET").on_press(Message::MethodSelected("GET".to_string())),
                button("POST").on_press(Message::MethodSelected("POST".to_string())),
                button("PUT").on_press(Message::MethodSelected("PUT".to_string())),
                button("DELETE").on_press(Message::MethodSelected("DELETE".to_string())),
                button("Send").on_press(Message::SendRequest)
            ]
            .spacing(10)
            .padding(10),
            response_content
        ]
        .into()
    }
}