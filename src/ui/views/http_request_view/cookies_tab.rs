use super::{CookieSnapshot, HttpRequestView, Message};
use crate::ui::views::cookie_manager::{BadgeKind, CookieManagerView};
use iced::widget::{button, column, container, row, rule, scrollable, text, text_input};
use iced::{Alignment, Color, Element, Length, Theme};
use iced_fonts::lucide;

impl HttpRequestView {
    pub(super) fn create_cookies_tab_content(&self) -> Element<'_, Message, Theme, iced::Renderer> {
        let manager = CookieManagerView {
            selected_domain: self.cookie_manager.selected_domain.clone(),
            search_query: self.cookie_manager.search_query.clone(),
            editing_cookie: self.cookie_manager.editing_cookie.clone(),
            edit_value: self.cookie_manager.edit_value.clone(),
            pending_delete_cookie: None,
            pending_clear_all: false,
            domains: self.cookie_domains.clone(),
            cookies: self.cookie_domain_cookies.iter().map(|c| {
                crate::ui::views::cookie_manager::CookieSnapshot {
                    name: c.name.clone(),
                    value: c.value.clone(),
                    domain: c.domain.clone(),
                    path: c.path.clone(),
                    secure: c.secure,
                    http_only: c.http_only,
                    same_site: c.same_site.clone(),
                    expires: c.expires.clone(),
                }
            }).collect(),
            total_count: self.cookie_count,
        };

        let mut domain_list = column![].spacing(2);
        for (domain, count) in &self.cookie_domains {
            let is_selected = manager.selected_domain.as_ref() == Some(domain);
            let domain_label = row![
                lucide::globe().size(12),
                text(format!(" {} ({})", domain, count)).size(13),
            ]
            .spacing(4)
            .align_y(Alignment::Center);

            let domain_button = if is_selected {
                button(domain_label)
                    .style(|theme: &Theme, status: button::Status| {
                        button::primary(theme, status)
                    })
                    .width(Length::Fill)
            } else {
                button(domain_label).width(Length::Fill)
            }
            .on_press(Message::CookieManagerMsg(
                crate::ui::views::cookie_manager::Message::DomainSelected(domain.clone()),
            ));

            let clear_btn = button(lucide::trash().size(11)).on_press(Message::CookieManagerMsg(
                crate::ui::views::cookie_manager::Message::ClearDomain(domain.clone()),
            ));

            domain_list = domain_list.push(
                row![domain_button, clear_btn]
                    .spacing(4)
                    .align_y(Alignment::Center),
            );
        }

        let domain_panel = container(
            column![
                text("Domains")
                    .size(14)
                    .color(Color::from_rgb(0.6, 0.6, 0.6)),
                scrollable(domain_list).height(Length::Fill),
                rule::horizontal(1),
                row![button(
                    row![lucide::trash().size(12), text(" Clear All").size(12)].spacing(4)
                )
                .on_press(Message::CookieManagerMsg(
                    crate::ui::views::cookie_manager::Message::ClearAll,
                ))]
                .spacing(8),
            ]
            .spacing(8)
            .padding(10),
        )
        .width(Length::FillPortion(1))
        .height(Length::Fill);

        let cookie_panel = self.render_cookie_table(&manager);

        row![domain_panel, cookie_panel]
            .spacing(1)
            .height(Length::Fill)
            .into()
    }

    fn render_cookie_table(
        &self,
        manager: &CookieManagerView,
    ) -> Element<'_, Message, Theme, iced::Renderer> {
        use crate::ui::views::cookie_manager::Message as CmMsg;

        let domain = match &manager.selected_domain {
            Some(d) => d.clone(),
            None => {
                return container(
                    text("Select a domain to view cookies")
                        .size(14)
                        .color(Color::from_rgb(0.5, 0.5, 0.5)),
                )
                .center_x(Length::Fill)
                .center_y(Length::Fill)
                .into();
            }
        };

        let search = manager.search_query.to_lowercase();
        let filtered: Vec<&CookieSnapshot> = self
            .cookie_domain_cookies
            .iter()
            .filter(|c| c.domain == domain)
            .filter(|c| {
                search.is_empty()
                    || c.name.to_lowercase().contains(&search)
                    || c.value.to_lowercase().contains(&search)
                    || c.path.to_lowercase().contains(&search)
            })
            .collect();

        let header = row![
            text("Name").size(12).color(Color::from_rgb(0.6, 0.6, 0.6)),
            text("Value").size(12).color(Color::from_rgb(0.6, 0.6, 0.6)),
            text("Path").size(12).color(Color::from_rgb(0.6, 0.6, 0.6)),
            text("Flags").size(12).color(Color::from_rgb(0.6, 0.6, 0.6)),
            text("").size(12),
        ]
        .spacing(8)
        .align_y(Alignment::Center);

        let mut rows = column![header.spacing(8)].spacing(4);

        if filtered.is_empty() {
            rows = rows.push(
                text("No cookies found")
                    .size(14)
                    .color(Color::from_rgb(0.5, 0.5, 0.5)),
            );
        } else {
            for cookie in &filtered {
                let is_editing = manager
                    .editing_cookie
                    .as_ref()
                    .map(|(d, n, p)| d == &cookie.domain && n == &cookie.name && p == &cookie.path)
                    .unwrap_or(false);

                let value_cell: Element<'_, Message, Theme, iced::Renderer> = if is_editing {
                    text_input("value", &manager.edit_value)
                        .on_input(|s| Message::CookieManagerMsg(CmMsg::EditValueChanged(s)))
                        .padding(4)
                        .width(Length::Fill)
                        .into()
                } else {
                    text(&cookie.value).size(12).into()
                };

                let badges = render_cookie_badges(cookie);

                let edit_save_btn = if is_editing {
                    button(lucide::check().size(11))
                        .on_press(Message::CookieManagerMsg(CmMsg::SaveEdit))
                } else {
                    button(lucide::pencil().size(11)).on_press(Message::CookieManagerMsg(
                        CmMsg::StartEdit(domain.clone(), cookie.name.clone(), cookie.path.clone()),
                    ))
                };

                let delete_btn = button(lucide::trash().size(11)).on_press(
                    Message::CookieManagerMsg(CmMsg::DeleteCookie(
                        domain.clone(),
                        cookie.name.clone(),
                        cookie.path.clone(),
                    )),
                );

                let row_widget = row![
                    text(&cookie.name).size(12),
                    value_cell,
                    text(&cookie.path)
                        .size(12)
                        .color(Color::from_rgb(0.5, 0.5, 0.5)),
                    badges,
                    row![edit_save_btn, delete_btn].spacing(4),
                ]
                .spacing(8)
                .align_y(Alignment::Center);

                rows = rows.push(row_widget);
            }
        }

        let total_cookies = self.cookie_count;
        let total_domains = self.cookie_domain_count;

        container(
            column![
                row![
                    text_input("Search cookies...", &manager.search_query)
                        .on_input(|s| Message::CookieManagerMsg(CmMsg::CookieSearchChanged(s)))
                        .padding(8)
                        .width(Length::Fill),
                    button(row![lucide::upload().size(12), text(" Import").size(12)].spacing(4))
                        .on_press(Message::CookieManagerMsg(CmMsg::ImportCookies)),
                    button(row![lucide::download().size(12), text(" Export").size(12)].spacing(4))
                        .on_press(Message::CookieManagerMsg(CmMsg::ExportCookies)),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
                text(format!(
                    "{} cookies across {} domains",
                    total_cookies, total_domains
                ))
                .size(12)
                .color(Color::from_rgb(0.5, 0.5, 0.5)),
                scrollable(rows).height(Length::Fill),
            ]
            .spacing(8)
            .padding(10),
        )
        .width(Length::FillPortion(3))
        .height(Length::Fill)
        .into()
    }
}

fn render_cookie_badges<'a>(
    cookie: &CookieSnapshot,
) -> Element<'a, Message, Theme, iced::Renderer> {
    let mut badges = row![].spacing(4);

    if cookie.secure {
        badges = badges.push(super::helpers::cookie_badge("S", BadgeKind::Secure));
    }
    if cookie.http_only {
        badges = badges.push(super::helpers::cookie_badge("H", BadgeKind::HttpOnly));
    }
    match cookie.same_site.as_str() {
        "Strict" => {
            badges = badges.push(super::helpers::cookie_badge(
                "St",
                BadgeKind::SameSiteStrict,
            ));
        }
        "None" => {
            badges = badges.push(super::helpers::cookie_badge("N", BadgeKind::SameSiteNone));
        }
        _ => {
            badges = badges.push(super::helpers::cookie_badge("L", BadgeKind::SameSiteLax));
        }
    }

    badges.into()
}
