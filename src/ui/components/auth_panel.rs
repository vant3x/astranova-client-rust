use crate::data::auth::{Auth, AuthType};
use iced::widget::{column, pick_list, text, text_input};
use iced::{Element, Renderer, Theme};

pub fn auth_type_selector<'a, M: Clone + 'a>(
    current: AuthType,
    on_select: impl Fn(AuthType) -> M + 'a,
) -> Element<'a, M, Theme, Renderer> {
    pick_list(&AuthType::ALL[..], Some(current), on_select)
        .padding(10)
        .into()
}

pub fn basic_auth_inputs<'a, M: Clone + 'a>(
    user: &'a str,
    pass: &'a str,
    on_user: impl Fn(String) -> M + 'a,
    on_pass: impl Fn(String) -> M + 'a,
) -> Element<'a, M, Theme, Renderer> {
    column![
        text_input("Username", user)
            .on_input(on_user)
            .padding(10),
        text_input("Password", pass)
            .on_input(on_pass)
            .padding(10)
            .secure(true),
    ]
    .spacing(10)
    .into()
}

pub fn bearer_token_input<'a, M: Clone + 'a>(
    token: &'a str,
    on_change: impl Fn(String) -> M + 'a,
) -> Element<'a, M, Theme, Renderer> {
    column![text_input("Bearer Token", token)
        .on_input(on_change)
        .padding(10)
        .secure(true),]
    .spacing(10)
    .into()
}

pub fn api_key_inputs<'a, M: Clone + 'a>(
    key: &'a str,
    value: &'a str,
    location: crate::data::auth::ApiKeyLocation,
    on_key: impl Fn(String) -> M + 'a,
    on_value: impl Fn(String) -> M + 'a,
    on_location: impl Fn(crate::data::auth::ApiKeyLocation) -> M + 'a,
) -> Element<'a, M, Theme, Renderer> {
    column![
        text_input("Key Name", key)
            .on_input(on_key)
            .padding(10),
        text_input("Value", value)
            .on_input(on_value)
            .padding(10),
        pick_list(
            &crate::data::auth::ApiKeyLocation::ALL[..],
            Some(location),
            on_location,
        )
        .padding(10),
    ]
    .spacing(10)
    .into()
}

pub fn digest_auth_inputs<'a, M: Clone + 'a>(
    user: &'a str,
    pass: &'a str,
    on_user: impl Fn(String) -> M + 'a,
    on_pass: impl Fn(String) -> M + 'a,
) -> Element<'a, M, Theme, Renderer> {
    column![
        text("Digest Authentication").size(14),
        text_input("Username", user)
            .on_input(on_user)
            .padding(10),
        text_input("Password", pass)
            .on_input(on_pass)
            .padding(10)
            .secure(true),
    ]
    .spacing(10)
    .into()
}

#[allow(clippy::too_many_arguments)]
pub fn auth_panel<'a, M: Clone + 'a>(
    auth: &'a Auth,
    on_type_select: impl Fn(AuthType) -> M + 'a,
    on_bearer_token: impl Fn(String) -> M + 'a,
    on_basic_user: impl Fn(String) -> M + 'a,
    on_basic_pass: impl Fn(String) -> M + 'a,
    on_api_key_key: impl Fn(String) -> M + 'a,
    on_api_key_value: impl Fn(String) -> M + 'a,
    on_api_key_location: impl Fn(crate::data::auth::ApiKeyLocation) -> M + 'a,
    on_digest_user: impl Fn(String) -> M + 'a,
    on_digest_pass: impl Fn(String) -> M + 'a,
    oauth2_content: Element<'a, M, Theme, Renderer>,
) -> Element<'a, M, Theme, Renderer> {
    let current_auth_type = auth.auth_type();

    let auth_inputs = match auth {
        Auth::BearerToken(token) => {
            bearer_token_input(token, on_bearer_token)
        }
        Auth::Basic { user, pass } => {
            basic_auth_inputs(user, pass, on_basic_user, on_basic_pass)
        }
        Auth::ApiKey {
            key,
            value,
            location,
        } => {
            api_key_inputs(
                key,
                value,
                *location,
                on_api_key_key,
                on_api_key_value,
                on_api_key_location,
            )
        }
        Auth::Digest { user, pass } => {
            digest_auth_inputs(user, pass, on_digest_user, on_digest_pass)
        }
        Auth::OAuth2(_) => oauth2_content,
        Auth::None => {
            column![text("No authentication required.").size(14)]
                .spacing(10)
                .into()
        }
    };

    column![
        text("Authentication Type").size(16),
        auth_type_selector(current_auth_type, on_type_select),
        auth_inputs
    ]
    .spacing(15)
    .padding(20)
    .into()
}
