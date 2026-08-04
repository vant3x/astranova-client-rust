#![allow(dead_code)]

use iced::{
    widget::{column, container, row, text},
    Alignment, Border, Color, Element, Length, Renderer, Theme,
};
use iced_fonts::lucide;
use std::collections::VecDeque;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToastType {
    Success,
    Error,
    Warning,
    Info,
}

impl ToastType {
    pub fn color(&self) -> Color {
        match self {
            ToastType::Success => Color::from_rgb(0.2, 0.7, 0.3),
            ToastType::Error => Color::from_rgb(0.8, 0.2, 0.2),
            ToastType::Warning => Color::from_rgb(0.8, 0.7, 0.1),
            ToastType::Info => Color::from_rgb(0.2, 0.5, 0.8),
        }
    }

    pub fn icon_element(&self, size: f32) -> Element<'static, (), Theme, Renderer> {
        let icon = match self {
            ToastType::Success => lucide::circle_check(),
            ToastType::Error => lucide::circle_x(),
            ToastType::Warning => lucide::triangle_alert(),
            ToastType::Info => lucide::info(),
        };
        icon.size(size).color(self.color()).into()
    }
}

#[derive(Debug, Clone)]
pub struct Toast {
    pub message: String,
    pub toast_type: ToastType,
    pub created_at: Instant,
    pub duration: Duration,
}

impl Toast {
    pub fn success(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            toast_type: ToastType::Success,
            created_at: Instant::now(),
            duration: Duration::from_secs(3),
        }
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            toast_type: ToastType::Error,
            created_at: Instant::now(),
            duration: Duration::from_secs(5),
        }
    }

    pub fn warning(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            toast_type: ToastType::Warning,
            created_at: Instant::now(),
            duration: Duration::from_secs(4),
        }
    }

    pub fn info(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            toast_type: ToastType::Info,
            created_at: Instant::now(),
            duration: Duration::from_secs(3),
        }
    }

    pub fn is_expired(&self) -> bool {
        self.created_at.elapsed() >= self.duration
    }

    pub fn opacity(&self) -> f32 {
        let elapsed = self.created_at.elapsed();
        if elapsed >= self.duration {
            return 0.0;
        }
        let fade_duration = Duration::from_millis(500);
        let fade_start = self.duration.saturating_sub(fade_duration);
        if elapsed >= fade_start {
            let remaining = self.duration.saturating_sub(elapsed);
            remaining.as_millis() as f32 / fade_duration.as_millis() as f32
        } else {
            1.0
        }
    }
}

#[derive(Debug)]
pub struct ToastManager {
    pub toasts: VecDeque<Toast>,
    max_toasts: usize,
}

impl ToastManager {
    pub fn new() -> Self {
        Self {
            toasts: VecDeque::new(),
            max_toasts: 5,
        }
    }

    pub fn add(&mut self, toast: Toast) {
        if self.toasts.len() >= self.max_toasts {
            self.toasts.pop_front();
        }
        self.toasts.push_back(toast);
    }

    pub fn success(&mut self, message: impl Into<String>) {
        self.add(Toast::success(message));
    }

    pub fn error(&mut self, message: impl Into<String>) {
        self.add(Toast::error(message));
    }

    pub fn warning(&mut self, message: impl Into<String>) {
        self.add(Toast::warning(message));
    }

    pub fn info(&mut self, message: impl Into<String>) {
        self.add(Toast::info(message));
    }

    pub fn clean_expired(&mut self) {
        self.toasts.retain(|t| !t.is_expired());
    }

    pub fn view(&self, theme: &Theme) -> Element<'_, (), Theme, Renderer> {
        if self.toasts.is_empty() {
            return column![].into();
        }

        let mut toasts_column = column![].spacing(8);

        let (text_color, bg_color) = match theme {
            Theme::Dark => (
                Color::from_rgb(0.9, 0.9, 0.9),
                Color::from_rgb(0.15, 0.15, 0.15),
            ),
            Theme::Light => (
                Color::from_rgb(0.1, 0.1, 0.1),
                Color::from_rgb(0.95, 0.95, 0.95),
            ),
            _ => (
                Color::from_rgb(0.9, 0.9, 0.9),
                Color::from_rgb(0.15, 0.15, 0.15),
            ),
        };

        for toast in &self.toasts {
            let opacity = toast.opacity().clamp(0.0, 1.0);

            let icon_element = toast.toast_type.icon_element(16.0);

            let message_text = text(&toast.message).size(13).color(Color {
                a: opacity,
                ..text_color
            });

            let toast_content = row![icon_element, message_text,]
                .spacing(8)
                .align_y(Alignment::Center);

            let toast_element =
                container(toast_content)
                    .padding(12)
                    .max_width(400)
                    .style(move |_theme| container::Style {
                        background: Some(iced::Background::Color(Color {
                            a: opacity,
                            ..bg_color
                        })),
                        border: Border {
                            color: Color {
                                a: opacity,
                                ..toast.toast_type.color()
                            },
                            width: 1.0,
                            radius: 8.0.into(),
                        },
                        ..container::Style::default()
                    });

            toasts_column = toasts_column.push(toast_element);
        }

        container(toasts_column)
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(Alignment::End)
            .align_y(Alignment::Start)
            .padding(20)
            .into()
    }
}

impl Default for ToastManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toast_creation() {
        let toast = Toast::success("Test message");
        assert_eq!(toast.message, "Test message");
        assert_eq!(toast.toast_type, ToastType::Success);
        assert!(!toast.is_expired());
    }

    #[test]
    fn toast_types_colors() {
        assert_eq!(ToastType::Success.color(), Color::from_rgb(0.2, 0.7, 0.3));
        assert_eq!(ToastType::Error.color(), Color::from_rgb(0.8, 0.2, 0.2));
        assert_eq!(ToastType::Warning.color(), Color::from_rgb(0.8, 0.7, 0.1));
        assert_eq!(ToastType::Info.color(), Color::from_rgb(0.2, 0.5, 0.8));
    }

    #[test]
    fn toast_manager_add() {
        let mut manager = ToastManager::new();
        manager.success("Test");
        assert_eq!(manager.toasts.len(), 1);
    }

    #[test]
    fn toast_manager_max_limit() {
        let mut manager = ToastManager::new();
        manager.max_toasts = 3;
        for i in 0..5 {
            manager.success(format!("Toast {i}"));
        }
        assert_eq!(manager.toasts.len(), 3);
        // Should keep the last 3 (Toast 2, 3, 4)
        assert_eq!(manager.toasts[0].message, "Toast 2");
        assert_eq!(manager.toasts[1].message, "Toast 3");
        assert_eq!(manager.toasts[2].message, "Toast 4");
    }

    #[test]
    fn toast_expiry() {
        let mut toast = Toast::info("Test");
        toast.duration = Duration::from_millis(10);
        std::thread::sleep(Duration::from_millis(20));
        assert!(toast.is_expired());
    }

    #[test]
    fn toast_clean_expired() {
        let mut manager = ToastManager::new();
        manager.max_toasts = 10;
        // Add a toast that expires immediately
        let mut toast = Toast::info("Short lived");
        toast.duration = Duration::from_millis(5);
        manager.toasts.push_back(toast);
        manager.success("Long lived");
        assert_eq!(manager.toasts.len(), 2);
        std::thread::sleep(Duration::from_millis(10));
        manager.clean_expired();
        assert_eq!(manager.toasts.len(), 1);
        assert_eq!(manager.toasts[0].message, "Long lived");
    }

    #[test]
    fn toast_manager_fifo_order() {
        let mut manager = ToastManager::new();
        manager.max_toasts = 3;
        manager.success("First");
        manager.error("Second");
        manager.warning("Third");
        manager.info("Fourth");
        // "First" should be evicted, keeping Second, Third, Fourth
        assert_eq!(manager.toasts.len(), 3);
        assert_eq!(manager.toasts[0].message, "Second");
        assert_eq!(manager.toasts[1].message, "Third");
        assert_eq!(manager.toasts[2].message, "Fourth");
    }
}
