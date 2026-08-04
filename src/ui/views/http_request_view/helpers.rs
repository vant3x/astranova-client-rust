use super::{ContentType, Message};
use crate::ui::views::cookie_manager::BadgeKind;
use iced::widget::{container, text};
use iced::{Color, Element, Theme};

pub(super) fn content_type_to_syntax(ct: ContentType) -> &'static str {
    match ct {
        ContentType::Json => "json",
        ContentType::Html => "html",
        ContentType::Xml => "xml",
        ContentType::Text => "text",
    }
}

pub(super) fn response_content_type_to_syntax(ct: &str) -> &str {
    if ct.contains("json") {
        "json"
    } else if ct.contains("html") {
        "html"
    } else if ct.contains("xml") {
        "xml"
    } else if ct.contains("javascript") || ct.contains("ecmascript") {
        "javascript"
    } else if ct.contains("css") {
        "css"
    } else if ct.contains("yaml") || ct.contains("yml") {
        "yaml"
    } else if ct.contains("markdown") {
        "markdown"
    } else {
        "text"
    }
}

pub(super) fn cookie_badge(
    icon: &str,
    kind: BadgeKind,
) -> Element<'_, Message, Theme, iced::Renderer> {
    let bg = match kind {
        BadgeKind::Secure => Color::from_rgb(0.2, 0.6, 0.2),
        BadgeKind::HttpOnly => Color::from_rgb(0.3, 0.3, 0.7),
        BadgeKind::SameSiteStrict => Color::from_rgb(0.2, 0.5, 0.8),
        BadgeKind::SameSiteLax => Color::from_rgb(0.5, 0.5, 0.5),
        BadgeKind::SameSiteNone => Color::from_rgb(0.8, 0.5, 0.2),
    };

    container(text(icon).size(10).color(Color::WHITE))
        .style(move |_theme: &Theme| iced::widget::container::Style {
            background: Some(bg.into()),
            text_color: Some(Color::WHITE),
            border: iced::Border::default().rounded(4).color(bg).width(0),
            ..Default::default()
        })
        .padding(iced::Padding::from([2, 4]))
        .into()
}
