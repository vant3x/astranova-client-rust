use iced::Color;

pub fn method_color(method: &str) -> Color {
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

pub fn status_color(status: u16) -> Color {
    match status {
        200..=299 => Color::from_rgb(0.2, 0.7, 0.3),
        300..=399 => Color::from_rgb(0.2, 0.5, 0.8),
        400..=499 => Color::from_rgb(0.8, 0.5, 0.1),
        500..=599 => Color::from_rgb(0.8, 0.2, 0.2),
        _ => Color::from_rgb(0.5, 0.5, 0.5),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn method_colors_are_correct() {
        assert_eq!(method_color("GET"), Color::from_rgb(0.2, 0.7, 0.3));
        assert_eq!(method_color("POST"), Color::from_rgb(0.2, 0.4, 0.8));
        assert_eq!(method_color("PUT"), Color::from_rgb(0.8, 0.5, 0.1));
        assert_eq!(method_color("PATCH"), Color::from_rgb(0.8, 0.7, 0.1));
        assert_eq!(method_color("DELETE"), Color::from_rgb(0.8, 0.2, 0.2));
    }

    #[test]
    fn status_colors_are_correct() {
        assert_eq!(status_color(200), Color::from_rgb(0.2, 0.7, 0.3));
        assert_eq!(status_color(301), Color::from_rgb(0.2, 0.5, 0.8));
        assert_eq!(status_color(404), Color::from_rgb(0.8, 0.5, 0.1));
        assert_eq!(status_color(500), Color::from_rgb(0.8, 0.2, 0.2));
    }
}
