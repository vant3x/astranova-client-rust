use crate::cookie::CookieJar;
use crate::ui::theme::ThemeColors;
use iced::{
    widget::{button, column, container, row, rule, scrollable, text, text_input},
    Alignment, Element, Length, Theme,
};
use iced_fonts::lucide;

#[derive(Debug, Clone, Copy)]
pub enum BadgeKind {
    Secure,
    HttpOnly,
    SameSiteStrict,
    SameSiteLax,
    SameSiteNone,
}

#[derive(Debug, Clone)]
pub enum Message {
    DomainSelected(String),
    CookieSearchChanged(String),
    DeleteCookie(String, String, String),
    ClearDomain(String),
    ClearAll,
    StartEdit(String, String, String),
    EditValueChanged(String),
    SaveEdit,
    CancelEdit,
    ImportCookies,
    #[allow(dead_code)]
    ImportData(Option<String>),
    ExportCookies,
    #[allow(dead_code)]
    ExportComplete(Option<String>),
    #[allow(dead_code)]
    DeselectDomain,
    #[allow(dead_code)]
    RequestDeleteCookie(String, String, String),
    #[allow(dead_code)]
    ConfirmDeleteCookie(String, String, String),
    CancelDeleteCookie,
    #[allow(dead_code)]
    RequestClearAll,
    #[allow(dead_code)]
    ConfirmClearAll,
    CancelClearAll,
    Close,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct DomainCookies {
    pub domain: String,
    pub count: usize,
    pub cookies: Vec<CookieSnapshot>,
}

#[derive(Debug, Clone, Default)]
pub struct CookieSnapshot {
    pub name: String,
    pub value: String,
    #[allow(dead_code)]
    pub domain: String,
    pub path: String,
    pub secure: bool,
    pub http_only: bool,
    pub same_site: String,
    #[allow(dead_code)]
    pub expires: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct CookieManagerView {
    pub selected_domain: Option<String>,
    pub search_query: String,
    pub editing_cookie: Option<(String, String, String)>,
    pub edit_value: String,
    pub pending_delete_cookie: Option<(String, String, String)>,
    pub pending_clear_all: bool,
    pub domains: Vec<(String, usize)>,
    pub cookies: Vec<CookieSnapshot>,
    pub total_count: usize,
}

pub enum CookieManagerAction {
    DeleteCookie(String, String, String),
    ClearDomain(String),
    ClearAll,
    SaveEdit(String, String, String, String),
    ImportCookies,
    ExportCookies,
}

impl CookieManagerView {
    pub fn sync_from_jar(&mut self, jar: &CookieJar) {
        self.domains = jar
            .domains()
            .into_iter()
            .map(|(d, c)| (d.to_string(), c))
            .collect();
        self.total_count = jar.total_count();

        let domain_filter = self.selected_domain.clone();
        self.cookies = if let Some(ref domain) = domain_filter {
            jar.cookies_for_domain(domain)
                .into_iter()
                .map(|c| CookieSnapshot {
                    name: c.name.clone(),
                    value: c.value.clone(),
                    domain: c.domain.clone(),
                    path: c.path.clone(),
                    secure: c.secure,
                    http_only: c.http_only,
                    same_site: c.same_site.to_string(),
                    expires: c.expires.clone(),
                })
                .collect()
        } else {
            Vec::new()
        };
    }

    pub fn update(&mut self, message: Message) -> Option<CookieManagerAction> {
        match message {
            Message::DomainSelected(domain) => {
                self.selected_domain = Some(domain);
                self.editing_cookie = None;
                None
            }
            Message::CookieSearchChanged(query) => {
                self.search_query = query;
                None
            }
            Message::DeleteCookie(d, n, p) | Message::RequestDeleteCookie(d, n, p) => {
                Some(CookieManagerAction::DeleteCookie(d, n, p))
            }
            Message::ClearDomain(d) => Some(CookieManagerAction::ClearDomain(d)),
            Message::ClearAll | Message::ConfirmClearAll => Some(CookieManagerAction::ClearAll),
            Message::StartEdit(domain, name, path) => {
                self.editing_cookie = Some((domain, name, path));
                None
            }
            Message::EditValueChanged(value) => {
                self.edit_value = value;
                None
            }
            Message::SaveEdit => {
                if let Some((domain, name, path)) = self.editing_cookie.take() {
                    let new_value = self.edit_value.clone();
                    self.edit_value.clear();
                    Some(CookieManagerAction::SaveEdit(domain, name, path, new_value))
                } else {
                    None
                }
            }
            Message::CancelEdit => {
                self.editing_cookie = None;
                self.edit_value.clear();
                None
            }
            Message::ImportCookies => Some(CookieManagerAction::ImportCookies),
            Message::ImportData(_) => None,
            Message::ExportCookies => Some(CookieManagerAction::ExportCookies),
            Message::ExportComplete(_) => None,
            Message::DeselectDomain => {
                self.selected_domain = None;
                self.editing_cookie = None;
                None
            }
            Message::ConfirmDeleteCookie(_, _, _) => None,
            Message::CancelDeleteCookie => {
                self.pending_delete_cookie = None;
                None
            }
            Message::RequestClearAll => {
                self.pending_clear_all = true;
                None
            }
            Message::CancelClearAll => {
                self.pending_clear_all = false;
                None
            }
            Message::Close => None,
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let filtered_domains: Vec<_> = if self.search_query.is_empty() {
            self.domains.clone()
        } else {
            let q = self.search_query.to_lowercase();
            self.domains
                .iter()
                .filter(|(d, _)| d.to_lowercase().contains(&q))
                .cloned()
                .collect()
        };

        let domain_list: Element<Message> = if filtered_domains.is_empty() {
            column![text("No cookies stored")
                .size(13)
                .color(ThemeColors::TEXT_MUTED)]
            .spacing(8)
            .into()
        } else {
            let mut list = column![].spacing(2);
            for (domain, count) in &filtered_domains {
                let is_selected = self.selected_domain.as_deref() == Some(domain.as_str());
                let domain_btn = button(
                    row![
                        lucide::globe().size(12),
                        text(format!("{domain} ({count})")).size(13),
                    ]
                    .spacing(6)
                    .align_y(Alignment::Center),
                )
                .width(Length::Fill)
                .on_press(Message::DomainSelected(domain.clone()))
                .style(if is_selected {
                    |_theme: &Theme, _status: button::Status| button::Style {
                        background: Some(iced::Background::Color(
                            ThemeColors::ACCENT.scale_alpha(0.2),
                        )),
                        ..button::Style::default()
                    }
                } else {
                    |_theme: &Theme, _status: button::Status| button::Style::default()
                });

                list = list.push(domain_btn);
            }
            scrollable(list).height(Length::Fill).into()
        };

        let search_bar = text_input("Search domains...", &self.search_query)
            .on_input(Message::CookieSearchChanged)
            .size(13);

        let domain_panel = column![
            row![lucide::globe().size(14), text("Domains").size(14)]
                .spacing(6)
                .align_y(Alignment::Center),
            search_bar,
            domain_list,
            rule::horizontal(1),
            text(format!("{} total cookies", self.total_count))
                .size(11)
                .color(ThemeColors::TEXT_MUTED),
        ]
        .spacing(8)
        .width(Length::FillPortion(1));

        let detail_panel: Element<Message> = match &self.selected_domain {
            Some(domain) => {
                let search = self.search_query.to_lowercase();
                let filtered: Vec<&CookieSnapshot> = self
                    .cookies
                    .iter()
                    .filter(|c| {
                        search.is_empty()
                            || c.name.to_lowercase().contains(&search)
                            || c.value.to_lowercase().contains(&search)
                            || c.path.to_lowercase().contains(&search)
                    })
                    .collect();

                let mut cookie_list = column![].spacing(4);
                for cookie in &filtered {
                    let is_editing = self.editing_cookie.as_ref().is_some_and(|(d, n, p)| {
                        d == domain && n == &cookie.name && p == &cookie.path
                    });

                    let is_pending_delete =
                        self.pending_delete_cookie
                            .as_ref()
                            .is_some_and(|(d, n, p)| {
                                d == domain && n == &cookie.name && p == &cookie.path
                            });

                    let cookie_row = if is_editing {
                        row![
                            text_input("Cookie value", &self.edit_value)
                                .on_input(Message::EditValueChanged)
                                .size(12),
                            button(lucide::check().size(12)).on_press(Message::SaveEdit),
                            button(lucide::x().size(12)).on_press(Message::CancelEdit),
                        ]
                        .spacing(6)
                        .align_y(Alignment::Center)
                    } else if is_pending_delete {
                        row![
                            text("Delete?").size(12).color(ThemeColors::ERROR),
                            button(text("Yes").size(12)).on_press(Message::ConfirmDeleteCookie(
                                domain.clone(),
                                cookie.name.clone(),
                                cookie.path.clone(),
                            )),
                            button(lucide::x().size(12)).on_press(Message::CancelDeleteCookie),
                        ]
                        .spacing(6)
                        .align_y(Alignment::Center)
                    } else {
                        let mut badges = row![].spacing(4);
                        if cookie.secure {
                            badges = badges.push(render_badge("Secure", ThemeColors::SUCCESS));
                        }
                        if cookie.http_only {
                            badges = badges.push(render_badge("HttpOnly", ThemeColors::INFO));
                        }
                        let ss_color = match cookie.same_site.as_str() {
                            "Strict" => ThemeColors::PURPLE,
                            "Lax" => ThemeColors::ORANGE,
                            _ => ThemeColors::TEXT_MUTED,
                        };
                        badges =
                            badges.push(render_badge(format!("SS:{}", cookie.same_site), ss_color));

                        row![
                            column![
                                row![
                                    text(&cookie.name).size(13),
                                    text(" = ").size(12).color(ThemeColors::TEXT_MUTED),
                                    text(&cookie.value)
                                        .size(12)
                                        .color(ThemeColors::TEXT_SECONDARY),
                                ]
                                .spacing(4),
                                row![text(format!("Path: {}", cookie.path))
                                    .size(11)
                                    .color(ThemeColors::TEXT_MUTED),],
                                badges,
                            ]
                            .spacing(4)
                            .width(Length::Fill),
                            button(lucide::pencil().size(12)).on_press(Message::StartEdit(
                                domain.clone(),
                                cookie.name.clone(),
                                cookie.path.clone(),
                            )),
                            button(lucide::trash().size(12)).on_press(Message::DeleteCookie(
                                domain.clone(),
                                cookie.name.clone(),
                                cookie.path.clone(),
                            )),
                        ]
                        .spacing(8)
                        .align_y(Alignment::Center)
                    };

                    cookie_list = cookie_list.push(cookie_row);
                }

                let clear_domain_btn =
                    button(row![lucide::trash().size(12), text(" Clear domain")].spacing(4))
                        .on_press(Message::ClearDomain(domain.clone()));

                column![
                    row![lucide::cookie().size(14), text(domain.as_str()).size(14),]
                        .spacing(6)
                        .align_y(Alignment::Center),
                    scrollable(cookie_list).height(Length::Fill),
                    rule::horizontal(1),
                    clear_domain_btn,
                ]
                .spacing(8)
                .into()
            }
            None => column![text("Select a domain to view cookies")
                .size(13)
                .color(ThemeColors::TEXT_MUTED),]
            .spacing(8)
            .into(),
        };

        let clear_all_btn = if self.pending_clear_all {
            row![
                button(
                    text("Confirm clear all?")
                        .size(12)
                        .color(ThemeColors::ERROR)
                )
                .on_press(Message::ClearAll),
                button(lucide::x().size(12)).on_press(Message::CancelClearAll),
            ]
            .spacing(6)
        } else {
            row![
                button(row![lucide::trash().size(12), text(" Clear All")].spacing(4))
                    .on_press(Message::ClearAll)
            ]
        };

        let import_btn = button(row![lucide::upload().size(12), text(" Import")].spacing(4))
            .on_press(Message::ImportCookies);
        let export_btn = button(row![lucide::download().size(12), text(" Export")].spacing(4))
            .on_press(Message::ExportCookies);
        let close_btn =
            button(row![lucide::x().size(12), text(" Close")].spacing(4)).on_press(Message::Close);

        let action_bar = row![import_btn, export_btn, clear_all_btn, close_btn]
            .spacing(10)
            .align_y(Alignment::Center);

        let content = column![
            row![lucide::cookie().size(16), text("Cookie Manager").size(16)]
                .spacing(6)
                .align_y(Alignment::Center),
            rule::horizontal(1),
            row![
                domain_panel,
                container(detail_panel)
                    .width(Length::FillPortion(2))
                    .height(Length::Fill)
            ]
            .spacing(16)
            .height(Length::Fill),
            rule::horizontal(1),
            action_bar,
        ]
        .spacing(12)
        .padding(20)
        .height(Length::Fill);

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}

fn render_badge(label: impl Into<String>, color: iced::Color) -> Element<'static, Message> {
    let label = label.into();
    container(text(label).size(10).color(color))
        .padding(iced::Padding::from([2, 6]))
        .style(move |_theme| container::Style {
            background: Some(iced::Background::Color(color.scale_alpha(0.15))),
            border: iced::Border {
                color: color.scale_alpha(0.3),
                width: 1.0,
                radius: 4.0.into(),
            },
            ..container::Style::default()
        })
        .into()
}
