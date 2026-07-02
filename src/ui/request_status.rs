use iced::Color;

#[derive(Debug, Default)]
pub enum RequestStatus {
    #[default]
    Idle,
    Loading,
    Success,
    Error(String),
}

impl Clone for RequestStatus {
    fn clone(&self) -> Self {
        match self {
            RequestStatus::Idle => RequestStatus::Idle,
            RequestStatus::Loading => RequestStatus::Loading,
            RequestStatus::Success => RequestStatus::Success,
            RequestStatus::Error(s) => RequestStatus::Error(s.clone()),
        }
    }
}

pub fn status_color(status: u16) -> Color {
    match status {
        200..=299 => Color::from_rgb(0.2, 0.7, 0.3),
        300..=399 => Color::from_rgb(0.2, 0.5, 0.8),
        400..=499 => Color::from_rgb(0.8, 0.5, 0.1),
        500..=599 => Color::from_rgb(0.8, 0.2, 0.2),
        _ => Color::from_rgb(0.5, 0.5, 0.5),
    }
}
