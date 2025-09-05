use iced::{
    widget::{button, column, row, text},
    Element, Length, Task,
};
use iced_aw::{TabLabel, Tabs};

use super::views::http_request_view::{self, HttpRequestView};
use crate::http_client::client;

pub fn main() -> iced::Result {
    iced::application("AstraNova Client", update, view).run()
}

struct AstraNovaApp {
    request_tabs: Vec<HttpRequestView>,
    active_request_tab_index: usize,
}

impl Default for AstraNovaApp {
    fn default() -> Self {
        Self {
            request_tabs: vec![HttpRequestView::default()],
            active_request_tab_index: 0,
        }
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    HttpRequestViewMsg(usize, http_request_view::Message),
    AddRequestTab,
    CloseRequestTab(usize),
    SelectRequestTab(usize),
}

fn update(app: &mut AstraNovaApp, message: Message) -> Task<Message> {
    match message {
        Message::HttpRequestViewMsg(index, msg) => {
            if let Some(view) = app.request_tabs.get_mut(index) {
                if let http_request_view::Message::SendRequest(request) = msg {
                    view.update(http_request_view::Message::SetLoading);

                    return Task::perform(
                        async move { client::send_request(request).await },
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
            app.request_tabs.push(HttpRequestView::default());
            app.active_request_tab_index = app.request_tabs.len() - 1;
        }
        Message::CloseRequestTab(index) => {
            if app.request_tabs.len() > 1 {
                app.request_tabs.remove(index);
                if app.active_request_tab_index >= app.request_tabs.len() {
                    app.active_request_tab_index = app.request_tabs.len() - 1;
                }
            }
        }
        Message::SelectRequestTab(index) => {
            app.active_request_tab_index = index;
        }
    }
    Task::none()
}

fn view(app: &AstraNovaApp) -> Element<'_, Message> {
    let mut tabs = Tabs::new(Message::SelectRequestTab);

    for (index, request_tab) in app.request_tabs.iter().enumerate() {
        let tab_label = if request_tab.url_input.is_empty() {
            TabLabel::Text(format!("New Request {}", index + 1))
        } else {
            TabLabel::Text(request_tab.url_input.clone())
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
        .set_active_tab(&app.active_request_tab_index)
        .width(Length::Fill)
        .height(Length::Fill);

    let add_tab_button = button(text("+")).on_press(Message::AddRequestTab);
    let close_tab_button = if app.request_tabs.len() > 1 {
        button(text("x")).on_press(Message::CloseRequestTab(app.active_request_tab_index))
    } else {
        button(text("x"))
    };

    column![
        row![add_tab_button, close_tab_button]
            .spacing(10)
            .padding(10),
        tabs_widget,
    ]
    .into()
}
