use crate::data::auth::Auth;
use crate::ui::app::{AstraNovaApp, Message};
use iced::Task;

pub fn handle_start_auth(app: &AstraNovaApp, index: usize) -> Task<Message> {
    if let Some(view) = app.request_tabs.get(index) {
        if let Auth::OAuth2(config) = &view.auth {
            let pkce = if config.pkce_enabled {
                Some(crate::data::oauth2::PKCEChallenge::generate())
            } else {
                None
            };

            let state = crate::data::oauth2::generate_state();
            let auth_url = crate::data::oauth2::build_authorization_url(
                &config.auth_url,
                &config.client_id,
                &config.redirect_uri,
                &config.scopes,
                pkce.as_ref(),
                &state,
            );

            let verifier = pkce.map(|p| p.verifier);

            return Task::perform(
                async move {
                    let _ = open::that(&auth_url);
                    verifier.ok_or_else(|| "No PKCE verifier".to_string())
                },
                move |result| Message::OAuth2AuthComplete(index, result),
            );
        }
    }
    Task::none()
}

pub fn handle_auth_complete(
    app: &mut AstraNovaApp,
    index: usize,
    result: Result<String, String>,
) -> Task<Message> {
    if let Some(view) = app.request_tabs.get_mut(index) {
        if let Auth::OAuth2(config) = &mut view.auth {
            match result {
                Ok(code) => {
                    let token_url = config.token_url.clone();
                    let client_id = config.client_id.clone();
                    let client_secret = config.client_secret.clone();
                    let redirect_uri = config.redirect_uri.clone();
                    let pkce_verifier = config.access_token.clone();
                    let tab_index = index;

                    return Task::perform(
                        async move {
                            crate::data::oauth2::exchange_code(
                                &token_url,
                                &code,
                                &client_id,
                                &client_secret,
                                &redirect_uri,
                                Some(&pkce_verifier),
                            )
                            .await
                        },
                        move |result| Message::OAuth2TokenReceived(tab_index, result),
                    );
                }
                Err(e) => {
                    log::error!("OAuth2 authorization failed: {}", e);
                }
            }
        }
    }
    Task::none()
}

pub fn handle_token_received(
    app: &mut AstraNovaApp,
    index: usize,
    result: Result<crate::data::oauth2::OAuth2TokenResponse, String>,
) -> Task<Message> {
    if let Some(view) = app.request_tabs.get_mut(index) {
        if let Auth::OAuth2(config) = &mut view.auth {
            match result {
                Ok(token_response) => {
                    config.access_token = token_response.access_token;
                    if let Some(refresh) = token_response.refresh_token {
                        config.refresh_token = refresh;
                    }
                    log::info!("OAuth2 token received successfully");
                }
                Err(e) => {
                    log::error!("OAuth2 token exchange failed: {}", e);
                }
            }
        }
    }
    Task::none()
}

pub fn handle_refresh_token(app: &mut AstraNovaApp, index: usize) -> Task<Message> {
    if let Some(view) = app.request_tabs.get(index) {
        if let Auth::OAuth2(config) = &view.auth {
            if !config.device_code.is_empty() {
                let token_url = config.token_url.clone();
                let device_code = config.device_code.clone();
                let client_id = config.client_id.clone();
                let client_secret = config.client_secret.clone();
                let tab_index = index;

                return Task::perform(
                    async move {
                        crate::data::oauth2::poll_device_token(
                            &token_url,
                            &device_code,
                            &client_id,
                            &client_secret,
                        )
                        .await
                    },
                    move |result| Message::OAuth2DeviceTokenPoll(tab_index, result),
                );
            } else if config.refresh_token.is_empty() {
                app.toast_manager
                    .warning("No refresh token available. Get a new token first.".to_string());
            } else {
                let token_url = config.token_url.clone();
                let refresh_token = config.refresh_token.clone();
                let client_id = config.client_id.clone();
                let client_secret = config.client_secret.clone();
                let tab_index = index;

                return Task::perform(
                    async move {
                        crate::data::oauth2::refresh_token(
                            &token_url,
                            &refresh_token,
                            &client_id,
                            &client_secret,
                        )
                        .await
                    },
                    move |result| Message::OAuth2TokenReceived(tab_index, result),
                );
            }
        }
    }
    Task::none()
}

pub fn handle_start_device_auth(app: &AstraNovaApp, index: usize) -> Task<Message> {
    if let Some(view) = app.request_tabs.get(index) {
        if let Auth::OAuth2(config) = &view.auth {
            if config.device_auth_url.is_empty() {
                log::warn!("No device authorization URL configured");
            } else {
                let device_auth_url = config.device_auth_url.clone();
                let client_id = config.client_id.clone();
                let scopes = config.scopes.clone();
                let tab_index = index;

                return Task::perform(
                    async move {
                        crate::data::oauth2::device_authorization(
                            &device_auth_url,
                            &client_id,
                            &scopes,
                        )
                        .await
                    },
                    move |result| Message::OAuth2DeviceAuthReceived(tab_index, result),
                );
            }
        }
    }
    Task::none()
}

pub fn handle_device_auth_received(
    app: &mut AstraNovaApp,
    index: usize,
    result: Result<crate::data::oauth2::DeviceAuthorizationResponse, String>,
) -> Task<Message> {
    if let Some(view) = app.request_tabs.get_mut(index) {
        if let Auth::OAuth2(config) = &mut view.auth {
            match result {
                Ok(device_auth) => {
                    config.device_code = device_auth.device_code;
                    config.user_code = device_auth.user_code;
                    config.verification_uri = device_auth.verification_uri;
                    config.device_code_expires_in = Some(device_auth.expires_in);
                    config.device_code_interval = device_auth.interval;

                    let verification_url = config.verification_uri.clone();
                    let user_code = config.user_code.clone();

                    log::info!(
                        "Device authorization received. User code: {}",
                        user_code
                    );

                    let _ = open::that(&verification_url);
                }
                Err(e) => {
                    log::error!("Device authorization failed: {}", e);
                }
            }
        }
    }
    Task::none()
}

pub fn handle_device_token_poll(
    app: &mut AstraNovaApp,
    index: usize,
    result: Result<crate::data::oauth2::DeviceTokenResponse, String>,
) -> Task<Message> {
    if let Some(view) = app.request_tabs.get_mut(index) {
        if let Auth::OAuth2(config) = &mut view.auth {
            match result {
                Ok(device_token) => {
                    if let Some(access_token) = device_token.access_token {
                        config.access_token = access_token;
                        if let Some(refresh) = device_token.refresh_token {
                            config.refresh_token = refresh;
                        }
                        config.device_code.clear();
                        config.user_code.clear();
                        config.verification_uri.clear();
                        log::info!("Device token received successfully");
                    } else if let Some(error) = device_token.error {
                        if error == "authorization_pending" {
                            log::info!("Authorization pending, polling again...");
                        } else if error == "slow_down" {
                            log::warn!("Slow down detected, increasing interval");
                        } else {
                            log::error!("Device token error: {}", error);
                            config.device_code.clear();
                            config.user_code.clear();
                            config.verification_uri.clear();
                        }
                    }
                }
                Err(e) => {
                    log::error!("Device token poll failed: {}", e);
                }
            }
        }
    }
    Task::none()
}
