use iced::Color;

// ── Modern Color Palette ─────────────────────────────────────────────

#[allow(dead_code)]
pub struct ThemeColors;

#[allow(dead_code)]
impl ThemeColors {
    // Background layers
    pub const BG_DARK: Color = Color::from_rgb(0.10, 0.10, 0.14);
    pub const BG_MEDIUM: Color = Color::from_rgb(0.13, 0.13, 0.17);
    pub const BG_LIGHT: Color = Color::from_rgb(0.16, 0.16, 0.20);
    pub const BG_HOVER: Color = Color::from_rgb(0.19, 0.19, 0.24);
    pub const BG_ACTIVE: Color = Color::from_rgb(0.22, 0.22, 0.28);

    // Text
    pub const TEXT_PRIMARY: Color = Color::from_rgb(0.92, 0.92, 0.94);
    pub const TEXT_SECONDARY: Color = Color::from_rgb(0.65, 0.65, 0.70);
    pub const TEXT_MUTED: Color = Color::from_rgb(0.45, 0.45, 0.50);
    pub const TEXT_DIM: Color = Color::from_rgb(0.35, 0.35, 0.40);

    // Borders
    pub const BORDER: Color = Color::from_rgb(0.22, 0.22, 0.28);
    pub const BORDER_LIGHT: Color = Color::from_rgb(0.28, 0.28, 0.34);
    pub const BORDER_FOCUS: Color = Color::from_rgb(0.30, 0.55, 0.90);

    // Accent
    pub const ACCENT: Color = Color::from_rgb(0.30, 0.55, 0.90);
    pub const ACCENT_HOVER: Color = Color::from_rgb(0.35, 0.60, 0.95);
    pub const ACCENT_DIM: Color = Color::from_rgb(0.20, 0.40, 0.70);

    // Semantic
    pub const SUCCESS: Color = Color::from_rgb(0.20, 0.72, 0.40);
    pub const SUCCESS_DIM: Color = Color::from_rgb(0.15, 0.50, 0.30);
    pub const WARNING: Color = Color::from_rgb(0.95, 0.75, 0.15);
    pub const WARNING_DIM: Color = Color::from_rgb(0.70, 0.55, 0.10);
    pub const ERROR: Color = Color::from_rgb(0.90, 0.30, 0.30);
    pub const ERROR_DIM: Color = Color::from_rgb(0.65, 0.20, 0.20);
    pub const INFO: Color = Color::from_rgb(0.30, 0.60, 0.95);
    pub const INFO_DIM: Color = Color::from_rgb(0.20, 0.40, 0.70);

    // Special
    pub const PURPLE: Color = Color::from_rgb(0.60, 0.40, 0.85);
    pub const PINK: Color = Color::from_rgb(0.85, 0.35, 0.60);
    pub const CYAN: Color = Color::from_rgb(0.20, 0.75, 0.85);
    pub const ORANGE: Color = Color::from_rgb(0.95, 0.55, 0.15);
}

// ── HTTP Method Colors (vibrant, distinguishable) ────────────────────

pub fn method_color(method: &str) -> Color {
    match method {
        "GET" => Color::from_rgb(0.20, 0.72, 0.40),
        "POST" => Color::from_rgb(0.30, 0.55, 0.90),
        "PUT" => Color::from_rgb(0.95, 0.55, 0.15),
        "PATCH" => Color::from_rgb(0.85, 0.65, 0.15),
        "DELETE" => Color::from_rgb(0.90, 0.30, 0.30),
        "HEAD" => Color::from_rgb(0.55, 0.55, 0.60),
        "OPTIONS" => Color::from_rgb(0.45, 0.55, 0.65),
        _ => Color::from_rgb(0.55, 0.55, 0.60),
    }
}

/// Dimmer version of method color for badges/backgrounds
pub fn method_color_dim(method: &str) -> Color {
    match method {
        "GET" => Color::from_rgb(0.12, 0.42, 0.24),
        "POST" => Color::from_rgb(0.15, 0.30, 0.55),
        "PUT" => Color::from_rgb(0.55, 0.32, 0.08),
        "PATCH" => Color::from_rgb(0.50, 0.38, 0.08),
        "DELETE" => Color::from_rgb(0.55, 0.18, 0.18),
        "HEAD" => Color::from_rgb(0.32, 0.32, 0.35),
        "OPTIONS" => Color::from_rgb(0.28, 0.34, 0.40),
        _ => Color::from_rgb(0.32, 0.32, 0.35),
    }
}

// ── HTTP Status Colors ───────────────────────────────────────────────

pub fn status_color(status: u16) -> Color {
    match status {
        200..=299 => Color::from_rgb(0.20, 0.72, 0.40),
        300..=399 => Color::from_rgb(0.30, 0.60, 0.95),
        400..=499 => Color::from_rgb(0.95, 0.75, 0.15),
        500..=599 => Color::from_rgb(0.90, 0.30, 0.30),
        _ => Color::from_rgb(0.55, 0.55, 0.60),
    }
}

pub fn status_color_dim(status: u16) -> Color {
    match status {
        200..=299 => Color::from_rgb(0.12, 0.42, 0.24),
        300..=399 => Color::from_rgb(0.15, 0.35, 0.55),
        400..=499 => Color::from_rgb(0.55, 0.42, 0.08),
        500..=599 => Color::from_rgb(0.55, 0.18, 0.18),
        _ => Color::from_rgb(0.32, 0.32, 0.35),
    }
}

/// Human-readable status label
pub fn status_label(status: u16) -> &'static str {
    match status {
        200 => "OK",
        201 => "Created",
        204 => "No Content",
        301 => "Moved Permanently",
        302 => "Found",
        304 => "Not Modified",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        408 => "Request Timeout",
        409 => "Conflict",
        422 => "Unprocessable Entity",
        429 => "Too Many Requests",
        500 => "Internal Server Error",
        502 => "Bad Gateway",
        503 => "Service Unavailable",
        _ => "",
    }
}

// ── UI Helper Functions ──────────────────────────────────────────────

/// Method badge background color (dim version for backgrounds)
#[allow(dead_code)]
pub fn method_badge_bg(method: &str) -> Color {
    method_color_dim(method)
}

/// Status badge background color
#[allow(dead_code)]
pub fn status_badge_bg(status: u16) -> Color {
    status_color_dim(status)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn method_colors_are_correct() {
        assert_eq!(method_color("GET"), Color::from_rgb(0.20, 0.72, 0.40));
        assert_eq!(method_color("POST"), Color::from_rgb(0.30, 0.55, 0.90));
        assert_eq!(method_color("PUT"), Color::from_rgb(0.95, 0.55, 0.15));
        assert_eq!(method_color("PATCH"), Color::from_rgb(0.85, 0.65, 0.15));
        assert_eq!(method_color("DELETE"), Color::from_rgb(0.90, 0.30, 0.30));
    }

    #[test]
    fn status_colors_are_correct() {
        assert_eq!(status_color(200), Color::from_rgb(0.20, 0.72, 0.40));
        assert_eq!(status_color(301), Color::from_rgb(0.30, 0.60, 0.95));
        assert_eq!(status_color(404), Color::from_rgb(0.95, 0.75, 0.15));
        assert_eq!(status_color(500), Color::from_rgb(0.90, 0.30, 0.30));
    }

    #[test]
    fn status_labels() {
        assert_eq!(status_label(200), "OK");
        assert_eq!(status_label(404), "Not Found");
        assert_eq!(status_label(500), "Internal Server Error");
        assert_eq!(status_label(999), "");
    }

    #[test]
    fn dim_colors_are_darker() {
        let c = method_color("GET");
        let d = method_color_dim("GET");
        assert!(d.r < c.r || d.g < c.g || d.b < c.b);
    }
}
