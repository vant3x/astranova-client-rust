use iced::{widget::column, Element, Task};

use super::views::http_request_view::{self, HttpRequestView};
use crate::http_client::{client, request::HttpRequest};

pub fn main() -> iced::Result {
    iced::application("AstraNova Client", update, view).run()
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
                app.http_request_view
                    .update(http_request_view::Message::SetLoading);

                // dlegate request building to the view
                let request = app.http_request_view.build_request();

                return Task::perform(
                    async move { client::send_request(request).await },
                    |result| {
                        Message::HttpRequestViewMessage(
                            http_request_view::Message::ResponseReceived(result),
                        )
                    },
                );
            } else {
                app.http_request_view.update(msg);
            }
        }
    }
    Task::none()
}

fn view(app: &AstraNovaApp) -> Element<'_, Message> {
    column![app
        .http_request_view
        .view()
        .map(Message::HttpRequestViewMessage),]
    .into()
}
