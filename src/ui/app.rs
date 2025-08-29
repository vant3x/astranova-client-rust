use iced::{
    executor,
    widget::{column},
    Application, Command, Element, Settings, Theme,
};

use crate::http_client::{client, request::HttpRequest};
use super::views::http_request_view::{self, HttpRequestView};

pub fn main() -> iced::Result {
    let icon = {
        let image_bytes = include_bytes!("../../assets/logo-bg.png");
        // Use the module function instead of a method on Icon
        iced::window::icon::from_file_data(image_bytes, None)
            .expect("Failed to create icon from file data")
    };

    AstraNovaApp::run(Settings {
        window: iced::window::Settings {
            icon: Some(icon),
            ..iced::window::Settings::default()
        },
        ..Settings::default()
    })
}

struct AstraNovaApp {
    http_request_view: HttpRequestView,
}

#[derive(Debug, Clone)]
pub enum Message {
    HttpRequestViewMessage(http_request_view::Message),
}

impl Application for AstraNovaApp {
    type Executor = executor::Default;
    type Message = Message;
    type Theme = Theme;
    type Flags = ();

    fn new(_flags: ()) -> (AstraNovaApp, Command<Message>) {
        (AstraNovaApp {
            http_request_view: HttpRequestView::new(),
        }, Command::none())
    }

    fn title(&self) -> String {
        String::from("AstraNova Client ")
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::HttpRequestViewMessage(msg) => {
                match msg {
                    http_request_view::Message::SendRequest => {
                        let base_url = self.http_request_view.url_input.clone();
                        let params: Vec<(String, String)> = self.http_request_view.params_editor.entries.iter()
                            .filter(|p| !p.key.is_empty())
                            .map(|p| (p.key.clone(), p.value.clone()))
                            .collect();

                        let query_string = params.iter()
                            .map(|(k, v)| format!("{}={}", urlencoding::encode(k), urlencoding::encode(v)))
                            .collect::<Vec<String>>()
                            .join("&");

                        let final_url = if query_string.is_empty() {
                            base_url
                        } else if base_url.contains('?') {
                            format!("{}&{}", base_url, query_string)
                        } else {
                            format!("{}?{}", base_url, query_string)
                        };

                        let request = HttpRequest {
                            method: self.http_request_view.method.clone(),
                            url: final_url,
                            headers: self.http_request_view.headers_editor.entries.iter()
                                .filter(|h| !h.key.is_empty())
                                .map(|h| (h.key.clone(), h.value.clone()))
                                .collect(),
                            body: if self.http_request_view.body_input.is_empty() {
                                None
                            } else {
                                Some(self.http_request_view.body_input.clone())
                            },
                        };
                        return Command::perform(async move {
                            let result = client::send_request(request).await;
                            result
                        }, |result| Message::HttpRequestViewMessage(http_request_view::Message::ResponseReceived(result)));
                    }
                    http_request_view::Message::ResponseReceived(_) => {
                        println!("[APP] ResponseReceived event processed by App.");
                        self.http_request_view.update(msg);
                    }
                    _ => {
                        self.http_request_view.update(msg);
                    }
                }
            }
        }
        Command::none()
    }

    fn view(&self) -> Element<'_, Message> {
        column![
            self.http_request_view.view().map(Message::HttpRequestViewMessage),
        ]
        .into()
    }
}