use crate::persistence::database::CollectionRequest;
use crate::ui::theme::ThemeColors;
use iced::{
    widget::{button, column, container, progress_bar, row, scrollable, text},
    Alignment, Element, Length, Padding, Renderer, Theme,
};
use iced_fonts::lucide;

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum Message {
    StartRun(i32, String, Vec<CollectionRequest>),
    RequestCompleted(RequestRunResult),
    RunCompleted(Vec<RequestRunResult>),
    ToggleStopOnFailure(bool),
    DelayChanged(String),
    Stop,
    Close,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct RequestRunResult {
    pub request_id: i32,
    pub name: String,
    pub method: String,
    pub url: String,
    pub status_code: Option<u16>,
    pub duration_ms: u64,
    pub passed: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub enum RunStatus {
    #[default]
    Pending,
    Running,
    Passed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone)]
pub struct CollectionRunnerState {
    pub collection_id: i32,
    pub collection_name: String,
    pub requests: Vec<CollectionRequest>,
    pub results: Vec<RequestRunResult>,
    pub current_index: usize,
    pub is_running: bool,
    pub is_cancelled: bool,
    pub stop_on_failure: bool,
    pub delay_ms: String,
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
}

impl Default for CollectionRunnerState {
    fn default() -> Self {
        Self {
            collection_id: 0,
            collection_name: String::new(),
            requests: Vec::new(),
            results: Vec::new(),
            current_index: 0,
            is_running: false,
            is_cancelled: false,
            stop_on_failure: false,
            delay_ms: "0".to_string(),
            total: 0,
            passed: 0,
            failed: 0,
        }
    }
}

impl CollectionRunnerState {
    pub fn new(
        collection_id: i32,
        collection_name: String,
        requests: Vec<CollectionRequest>,
    ) -> Self {
        let total = requests.len();
        Self {
            collection_id,
            collection_name,
            requests,
            results: Vec::new(),
            current_index: 0,
            is_running: true,
            is_cancelled: false,
            stop_on_failure: false,
            delay_ms: "0".to_string(),
            total,
            passed: 0,
            failed: 0,
        }
    }

    pub fn progress(&self) -> f32 {
        if self.total == 0 {
            return 0.0;
        }
        self.results.len() as f32 / self.total as f32
    }

    pub fn status_for_index(&self, idx: usize) -> RunStatus {
        if idx < self.results.len() {
            if self.results[idx].passed {
                RunStatus::Passed
            } else {
                RunStatus::Failed
            }
        } else if idx == self.current_index && self.is_running {
            RunStatus::Running
        } else if self.is_cancelled && idx >= self.results.len() {
            RunStatus::Cancelled
        } else {
            RunStatus::Pending
        }
    }

    pub fn view(&self) -> Element<'_, Message, Theme, Renderer> {
        let progress = self.progress();
        let completed = self.results.len();

        let header = row![
            lucide::play().size(14).color(ThemeColors::ACCENT),
            text(format!("Collection Runner: {}", self.collection_name))
                .size(13)
                .color(ThemeColors::TEXT_PRIMARY),
        ]
        .spacing(6)
        .align_y(Alignment::Center);

        let progress_row = row![
            progress_bar(0.0..=1.0, progress)
                .girth(Length::Fixed(8.0))
                .style(|_theme: &Theme| iced::widget::progress_bar::Style {
                    background: iced::Background::Color(ThemeColors::BG_DARK),
                    bar: iced::Background::Color(ThemeColors::ACCENT),
                    border: iced::Border::default()
                        .rounded(4)
                        .color(ThemeColors::BORDER)
                        .width(1),
                }),
            text(format!("{}/{}", completed, self.total))
                .size(11)
                .color(ThemeColors::TEXT_SECONDARY),
        ]
        .spacing(8)
        .align_y(Alignment::Center)
        .width(Length::Fill);

        let mut results_col = column![].spacing(2);
        for (idx, req) in self.requests.iter().enumerate() {
            let status = self.status_for_index(idx);
            results_col = results_col.push(self.render_request_row(req, idx, &status));
        }

        let results_scroll = scrollable(results_col).height(Length::Fixed(300.0));

        let controls = row![
            text("Delay (ms):")
                .size(11)
                .color(ThemeColors::TEXT_SECONDARY),
            iced::widget::text_input("0", &self.delay_ms)
                .on_input(Message::DelayChanged)
                .size(11)
                .padding(Padding::from([3, 6]))
                .width(Length::Fixed(60.0))
                .style(|_theme: &Theme, status: iced::widget::text_input::Status| {
                    let border_color = match status {
                        iced::widget::text_input::Status::Focused { .. } => ThemeColors::ACCENT,
                        _ => ThemeColors::BORDER,
                    };
                    iced::widget::text_input::Style {
                        background: iced::Background::Color(ThemeColors::BG_DARK),
                        border: iced::Border::default()
                            .rounded(4)
                            .color(border_color)
                            .width(1),
                        icon: ThemeColors::TEXT_MUTED,
                        placeholder: ThemeColors::TEXT_DIM,
                        value: ThemeColors::TEXT_PRIMARY,
                        selection: ThemeColors::ACCENT_DIM,
                    }
                }),
            iced::widget::checkbox::Checkbox::new(self.stop_on_failure)
                .label("Stop on failure")
                .on_toggle(Message::ToggleStopOnFailure)
                .size(12)
                .text_size(11)
                .style(|_theme: &Theme, _status: iced::widget::checkbox::Status| {
                    iced::widget::checkbox::Style {
                        background: iced::Background::Color(ThemeColors::BG_DARK),
                        icon_color: ThemeColors::ACCENT,
                        border: iced::Border::default()
                            .rounded(3)
                            .color(ThemeColors::BORDER)
                            .width(1),
                        text_color: Some(ThemeColors::TEXT_SECONDARY),
                    }
                }),
        ]
        .spacing(8)
        .align_y(Alignment::Center);

        let summary = if self.is_running {
            text("").size(11)
        } else {
            let summary_color = if self.failed == 0 {
                ThemeColors::SUCCESS
            } else {
                ThemeColors::ERROR
            };
            text(format!(
                "Total: {}  Passed: {}  Failed: {}",
                self.total, self.passed, self.failed
            ))
            .size(11)
            .color(summary_color)
        };

        let stop_btn = if self.is_running {
            button(
                row![lucide::square().size(11), text(" Stop").size(11)]
                    .spacing(2)
                    .align_y(Alignment::Center),
            )
            .padding(Padding::from([4, 8]))
            .on_press(Message::Stop)
        } else {
            button(
                row![lucide::square().size(11), text(" Stop").size(11)]
                    .spacing(2)
                    .align_y(Alignment::Center),
            )
            .padding(Padding::from([4, 8]))
        };

        let close_btn = button(
            row![lucide::x().size(11), text(" Close").size(11)]
                .spacing(2)
                .align_y(Alignment::Center),
        )
        .padding(Padding::from([4, 8]))
        .on_press(Message::Close);

        let bottom_row = row![stop_btn, summary, close_btn]
            .spacing(8)
            .align_y(Alignment::Center);

        container(
            column![header, progress_row, results_scroll, controls, bottom_row,]
                .spacing(8)
                .padding(12),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .style(|_theme: &Theme| iced::widget::container::Style {
            background: Some(iced::Background::Color(ThemeColors::BG_DARK)),
            border: iced::Border::default()
                .rounded(6)
                .color(ThemeColors::BORDER)
                .width(1),
            ..Default::default()
        })
        .into()
    }

    fn render_request_row(
        &self,
        req: &CollectionRequest,
        idx: usize,
        status: &RunStatus,
    ) -> Element<'_, Message, Theme, Renderer> {
        let method_color = crate::ui::theme::method_color(&req.method);

        let (icon, status_color, status_text) = match status {
            RunStatus::Pending => (
                lucide::circle().size(10).color(ThemeColors::TEXT_MUTED),
                ThemeColors::TEXT_MUTED,
                "---",
            ),
            RunStatus::Running => (
                lucide::loader().size(10).color(ThemeColors::ACCENT),
                ThemeColors::ACCENT,
                "...",
            ),
            RunStatus::Passed => (
                lucide::circle_check().size(10).color(ThemeColors::SUCCESS),
                ThemeColors::SUCCESS,
                "Pass",
            ),
            RunStatus::Failed => (
                lucide::circle_x().size(10).color(ThemeColors::ERROR),
                ThemeColors::ERROR,
                "Fail",
            ),
            RunStatus::Cancelled => (
                lucide::ban().size(10).color(ThemeColors::TEXT_MUTED),
                ThemeColors::TEXT_MUTED,
                "Skip",
            ),
        };

        let result_info = if idx < self.results.len() {
            let r = &self.results[idx];
            let status_str = r
                .status_code
                .map_or_else(|| "ERR".to_string(), |s| s.to_string());
            let duration_str = format!("{}ms", r.duration_ms);
            let error_str = r.error.as_deref().unwrap_or("");

            row![
                text(status_str).size(10).color(status_color),
                text(duration_str).size(10).color(ThemeColors::TEXT_MUTED),
                if error_str.is_empty() {
                    text("").size(9)
                } else {
                    text(error_str).size(9).color(ThemeColors::ERROR)
                },
            ]
            .spacing(6)
            .align_y(Alignment::Center)
        } else {
            row![].spacing(6)
        };

        row![
            icon,
            text(req.method.clone()).size(10).color(method_color),
            text(req.name.clone())
                .size(11)
                .color(ThemeColors::TEXT_PRIMARY),
            result_info,
            text(status_text).size(10).color(status_color),
        ]
        .spacing(6)
        .align_y(Alignment::Center)
        .width(Length::Fill)
        .into()
    }
}
