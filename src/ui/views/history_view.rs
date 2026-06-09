use crate::persistence::database::RequestHistoryEntry;
use crate::ui::theme;
use iced::{
    widget::{button, column, container, row, scrollable, text, text_input},
    Alignment, Color, Element, Length, Renderer, Theme,
};
use iced_fonts::lucide;

#[derive(Debug, Clone)]
pub enum Message {
    ResendEntry(i32),
    ClearHistory,
    SearchChanged(String),
    FilterMethod(String),
    ExportHistory,
}

#[derive(Debug, Default)]
pub struct HistoryView {
    pub entries: Vec<RequestHistoryEntry>,
    pub selected_index: Option<usize>,
    pub search_query: String,
    pub filter_method: String,
}

impl Clone for HistoryView {
    fn clone(&self) -> Self {
        Self {
            entries: self.entries.clone(),
            selected_index: self.selected_index,
            search_query: self.search_query.clone(),
            filter_method: self.filter_method.clone(),
        }
    }
}

impl HistoryView {
    pub fn new() -> Self {
        Self::default()
    }

    fn filtered_entries(&self) -> Vec<&RequestHistoryEntry> {
        self.entries
            .iter()
            .filter(|e| {
                let matches_search = if self.search_query.is_empty() {
                    true
                } else {
                    let q = self.search_query.to_lowercase();
                    e.url.to_lowercase().contains(&q)
                        || e.method.to_lowercase().contains(&q)
                        || e.request_data
                            .as_ref()
                            .map(|d| d.to_lowercase().contains(&q))
                            .unwrap_or(false)
                };
                let matches_method = if self.filter_method.is_empty() {
                    true
                } else {
                    e.method.eq_ignore_ascii_case(&self.filter_method)
                };
                matches_search && matches_method
            })
            .collect()
    }

    pub fn update(&mut self, message: Message) -> Option<i32> {
        match message {
            Message::ResendEntry(entry_id) => Some(entry_id),
            Message::ClearHistory => {
                self.entries.clear();
                self.selected_index = None;
                self.search_query.clear();
                self.filter_method.clear();
                None
            }
            Message::SearchChanged(query) => {
                self.search_query = query;
                None
            }
            Message::FilterMethod(method) => {
                if self.filter_method == method {
                    self.filter_method.clear();
                } else {
                    self.filter_method = method;
                }
                None
            }
            Message::ExportHistory => None,
        }
    }

    pub fn view(&self) -> Element<'_, Message, Theme, Renderer> {
        let clear_button: Element<'_, Message, Theme, Renderer> = if self.entries.is_empty() {
            button(row![lucide::trash().size(14), text(" Clear")].spacing(4)).into()
        } else {
            button(row![lucide::trash().size(14), text(" Clear")].spacing(4))
                .on_press(Message::ClearHistory)
                .into()
        };

        let export_button: Element<'_, Message, Theme, Renderer> = if self.entries.is_empty() {
            button(row![lucide::download().size(14), text(" Export")].spacing(4)).into()
        } else {
            button(row![lucide::download().size(14), text(" Export")].spacing(4))
                .on_press(Message::ExportHistory)
                .into()
        };

        let header = row![text("History").size(16), clear_button, export_button]
            .spacing(10)
            .align_y(Alignment::Center);

        let search_input = text_input("Search by URL, method, body...", &self.search_query)
            .on_input(Message::SearchChanged)
            .padding(8)
            .width(Length::Fill);

        let methods = ["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS"];
        let mut filter_buttons = row![].spacing(4);
        for method in methods {
            let is_active = self.filter_method == method;
            let btn = if is_active {
                button(text(method).size(11))
                    .style(button::secondary)
                    .on_press(Message::FilterMethod(method.to_string()))
            } else {
                button(text(method).size(11)).on_press(Message::FilterMethod(method.to_string()))
            };
            filter_buttons = filter_buttons.push(btn);
        }

        let filter_row = row![
            text("Filter:").size(12),
            filter_buttons,
            text(format!(
                "({}/{})",
                self.filtered_entries().len(),
                self.entries.len()
            ))
            .size(11)
            .color(Color::from_rgb(0.5, 0.5, 0.5)),
        ]
        .spacing(8)
        .align_y(Alignment::Center);

        if self.entries.is_empty() {
            return container(
                column![
                    header,
                    search_input,
                    text("No request history yet.").size(14),
                ]
                .spacing(10)
                .padding(10),
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .into();
        }

        let filtered = self.filtered_entries();
        let mut list = column![].spacing(4);

        for entry in &filtered {
            let method_color = theme::method_color(&entry.method);

            let status_text = match entry.status {
                Some(s) => format!(" {}", s),
                None => " ---".to_string(),
            };

            let status_color = theme::status_color(entry.status.unwrap_or(0));

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

            let has_body = entry
                .request_data
                .as_ref()
                .map(|d| d.contains("\"body\":"))
                .unwrap_or(false);
            let has_auth = entry
                .request_data
                .as_ref()
                .map(|d| d.contains("\"auth_type\":"))
                .unwrap_or(false);
            let has_multipart = entry
                .request_data
                .as_ref()
                .map(|d| d.contains("multipart"))
                .unwrap_or(false);

            let mut indicators = row![].spacing(4);
            if has_body {
                indicators =
                    indicators.push(text("B").size(10).color(Color::from_rgb(0.3, 0.7, 0.9)));
            }
            if has_auth {
                indicators =
                    indicators.push(text("A").size(10).color(Color::from_rgb(0.8, 0.5, 0.1)));
            }
            if has_multipart {
                indicators =
                    indicators.push(text("M").size(10).color(Color::from_rgb(0.5, 0.3, 0.8)));
            }

            let entry_row = row![
                text(&entry.method).size(12).color(method_color),
                text(url_truncated).size(12),
                indicators,
                text(status_text).size(12).color(status_color),
                text(duration_text)
                    .size(12)
                    .color(Color::from_rgb(0.5, 0.5, 0.5)),
            ]
            .spacing(8)
            .align_y(Alignment::Center);

            let entry_button: Element<'_, Message, Theme, Renderer> = button(entry_row)
                .on_press(Message::ResendEntry(entry.id))
                .into();

            list = list.push(entry_button);
        }

        if filtered.is_empty() && !self.search_query.is_empty() {
            list = list.push(
                text(format!("No results for \"{}\"", self.search_query))
                    .size(13)
                    .color(Color::from_rgb(0.5, 0.5, 0.5)),
            );
        }

        container(
            column![header, search_input, filter_row, scrollable(list)]
                .spacing(8)
                .padding(10),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }
}
