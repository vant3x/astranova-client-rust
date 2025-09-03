use iced::{widget::column, Element, Task};

use crate::http_client::{client, request::HttpRequest};
use super::views::http_request_view::{self, HttpRequestView};

pub fn main() -> iced::Result {
    iced::application("AstraNova Client", update, view)
        .run()
}

#[derive(Default)]
struct AstraNovaApp {
    http_request_view: HttpRequestView,
}

#[derive(Debug, Clone)]
pub enum Message {
    HttpRequestViewMessage(http_request_view::Message),
}

fn update(app: &mut AstraNovaApp, message: Message) -> Task<Message> {
    match message {
        Message::HttpRequestViewMessage(msg) => {
            if let http_request_view::Message::SendRequest = msg {
                let base_url = app.http_request_view.url_input.clone();
                let params: Vec<(String, String)> = app.http_request_view.params_editor.entries.iter()
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
                    method: app.http_request_view.method.to_string(),
                    url: final_url,
                    headers: app.http_request_view.headers_editor.entries.iter()
                        .filter(|h| !h.key.is_empty())
                        .map(|h| (h.key.clone(), h.value.clone()))
                        .collect(),
                    body: if app.http_request_view.body_input.is_empty() {
                        None
                    } else {
                        Some(app.http_request_view.body_input.clone())
                    },
                };
                
                app.http_request_view.update(msg.clone());

                return Task::perform(async move {
                    client::send_request(request).await
                }, |result| Message::HttpRequestViewMessage(http_request_view::Message::ResponseReceived(result)));
            } else {
                app.http_request_view.update(msg);
            }
        }
    }
    Task::none()
}

fn view(app: &AstraNovaApp) -> Element<'_, Message> {
    column![
        app.http_request_view.view().map(Message::HttpRequestViewMessage),
    ]
    .into()
}

