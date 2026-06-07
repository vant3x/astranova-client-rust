use crate::persistence::database::RequestHistoryEntry;
use iced::{
    widget::{button, column, container, row, scrollable, text},
    Alignment, Color, Element, Length, Renderer, Theme,
};

#[derive(Debug, Clone)]
pub enum Message {
    SelectEntry(usize),
    ResendEntry(i32),
    ClearHistory,
}

#[derive(Debug, Default)]
pub struct HistoryView {
    pub entries: Vec<RequestHistoryEntry>,
    pub selected_index: Option<usize>,
}

impl Clone for HistoryView {
    fn clone(&self) -> Self {
        Self {
            entries: self.entries.clone(),
            selected_index: self.selected_index,
        }
    }
}

impl HistoryView {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn update(&mut self, message: Message) -> Option<i32> {
        match message {
            Message::SelectEntry(index) => {
                self.selected_index = Some(index);
                self.entries.get(index).map(|e| e.id)
            }
            Message::ResendEntry(entry_id) => {
                Some(entry_id)
            }
            Message::ClearHistory => {
                self.entries.clear();
                self.selected_index = None;
                None
            }
        }
    }

    pub fn view(&self) -> Element<'_, Message, Theme, Renderer> {
        let clear_button: Element<'_, Message, Theme, Renderer> = if self.entries.is_empty() {
            button("Clear").into()
        } else {
            button("Clear").on_press(Message::ClearHistory).into()
        };

        let header = row![
            text("History").size(16),
            clear_button,
        ]
        .spacing(10)
        .align_y(Alignment::Center);

        if self.entries.is_empty() {
            return container(
                column![
                    header,
                    text("No request history yet.").size(14),
                ]
                .spacing(10)
                .padding(10),
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .into();
        }

        let mut list = column![].spacing(4);

        for (index, entry) in self.entries.iter().enumerate() {
            let is_selected = self.selected_index == Some(index);
            let method_color = method_color(&entry.method);

            let status_text = match entry.status {
                Some(s) => format!(" {}", s),
                None => " ---".to_string(),
            };

            let status_color = match entry.status {
                Some(200..=299) => Color::from_rgb(0.2, 0.7, 0.3),
                Some(300..=399) => Color::from_rgb(0.2, 0.5, 0.8),
                Some(400..=499) => Color::from_rgb(0.8, 0.5, 0.1),
                Some(500..=599) => Color::from_rgb(0.8, 0.2, 0.2),
                _ => Color::from_rgb(0.5, 0.5, 0.5),
            };

            let duration_text = match entry.duration_ms {
                Some(d) => format!("{}ms", d),
                None => "N/A".to_string(),
            };

            let url_display: String = entry.url.chars().take(40).collect();
            let url_truncated = if entry.url.len() > 40 {
                format!("{}...", url_display)
            } else {
                url_display
            };

            let has_body = entry.request_data.as_ref().map(|d| d.contains("\"body\":")).unwrap_or(false);
            let has_auth = entry.request_data.as_ref().map(|d| d.contains("\"auth_type\":")).unwrap_or(false);
            let has_multipart = entry.request_data.as_ref().map(|d| d.contains("multipart")).unwrap_or(false);

            let mut indicators = row![].spacing(4);
            if has_body {
                indicators = indicators.push(text("B").size(10).color(Color::from_rgb(0.3, 0.7, 0.9)));
            }
            if has_auth {
                indicators = indicators.push(text("A").size(10).color(Color::from_rgb(0.8, 0.5, 0.1)));
            }
            if has_multipart {
                indicators = indicators.push(text("M").size(10).color(Color::from_rgb(0.5, 0.3, 0.8)));
            }

            let entry_row = row![
                text(&entry.method).size(12).color(method_color),
                text(url_truncated).size(12),
                indicators,
                text(status_text).size(12).color(status_color),
                text(duration_text).size(12).color(Color::from_rgb(0.5, 0.5, 0.5)),
            ]
            .spacing(8)
            .align_y(Alignment::Center);

            let entry_button: Element<'_, Message, Theme, Renderer> = if is_selected {
                button(entry_row).style(button::secondary).into()
            } else {
                button(entry_row).on_press(Message::ResendEntry(entry.id)).into()
            };

            list = list.push(entry_button);
        }

        container(column![header, scrollable(list)].spacing(10).padding(10))
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}

fn method_color(method: &str) -> Color {
    match method {
        "GET" => Color::from_rgb(0.2, 0.7, 0.3),
        "POST" => Color::from_rgb(0.2, 0.4, 0.8),
        "PUT" => Color::from_rgb(0.8, 0.5, 0.1),
        "PATCH" => Color::from_rgb(0.8, 0.7, 0.1),
        "DELETE" => Color::from_rgb(0.8, 0.2, 0.2),
        "HEAD" => Color::from_rgb(0.5, 0.5, 0.5),
        "OPTIONS" => Color::from_rgb(0.6, 0.6, 0.6),
        _ => Color::from_rgb(0.5, 0.5, 0.5),
    }
}
