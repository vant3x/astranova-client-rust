use crate::data::auth::Auth;
use crate::error::AppError;
use crate::ui::app::{AstraioApp, Message};
use iced::Task;

pub fn handle_start_auth(app: &mut AstraioApp, index: usize) -> Task<Message> {
    if let Some(view) = app.request_tabs.get_mut(index) {
        if let Auth::OAuth2(config) = &mut view.auth {
            let pkce = if config.pkce_enabled {
                Some(crate::data::oauth2::PKCEChallenge::generate())
            } else {
                None
            };

            let state = crate::data::oauth2::generate_state();
            let pkce_verifier = pkce.as_ref().map(|p| p.verifier.clone());
            config.pkce_verifier = pkce_verifier.clone();

            if let Some(ref verifier) = pkce_verifier {
                let identifier = config.client_id.clone();
                if let Err(e) = app.secret_store.store_pkce_verifier(&identifier, verifier) {
                    log::warn!("Failed to store PKCE verifier in keyring: {e}");
                }
            }

            let auth_url = config.auth_url.clone();
            let client_id = config.client_id.clone();
            let scopes = config.scopes.clone();

            return Task::perform(
                async move {
                    let server = crate::data::oauth2::LocalAuthCallback::start().await?;
                    let url = crate::data::oauth2::build_authorization_url(
                        &auth_url,
                        &client_id,
                        &server.redirect_uri,
                        &scopes,
                        pkce.as_ref(),
                        &state,
                    );
                    let _ = open::that(&url);
                    let (code, _state) = server.wait_for_code(120).await?;
                    Ok::<_, AppError>(code)
                },
                move |result| Message::OAuth2AuthComplete(index, result, pkce_verifier),
            );
        }
    }
    Task::none()
}

pub fn handle_auth_complete(
    app: &mut AstraioApp,
    index: usize,
    result: Result<String, AppError>,
    pkce_verifier: Option<String>,
) -> Task<Message> {
    match result {
        Ok(code) => {
            if code.is_empty() {
                app.toast_manager
                    .warning("No authorization code received".to_string());
                return Task::none();
            }

            if let Some(view) = app.request_tabs.get(index) {
                if let Auth::OAuth2(config) = &view.auth {
                    let token_url = config.token_url.clone();
                    let client_id = config.client_id.clone();
                    let client_secret = config.client_secret.clone();
                    let redirect_uri = config.redirect_uri.clone();
                    let verifier = pkce_verifier.clone();
                    let tab_index = index;
                    let http_client = app.http_client.clone();

                    return Task::perform(
                        async move {
                            crate::data::oauth2::exchange_code(
                                &http_client,
                                &token_url,
                                &code,
                                &client_id,
                                &client_secret,
                                &redirect_uri,
                                verifier.as_deref(),
                            )
                            .await
                        },
                        move |result| Message::OAuth2TokenReceived(tab_index, result),
                    );
                }
            }
            app.toast_manager.error("Tab not found".to_string());
        }
        Err(e) => {
            app.toast_manager
                .error(format!("Authorization failed: {e}"));
        }
    }
    Task::none()
}

pub fn handle_token_received(
    app: &mut AstraioApp,
    index: usize,
    result: Result<crate::data::oauth2::OAuth2TokenResponse, AppError>,
) -> Task<Message> {
    if let Some(view) = app.request_tabs.get_mut(index) {
        if let Auth::OAuth2(config) = &mut view.auth {
            match result {
                Ok(token_response) => {
                    config.access_token = token_response.access_token.clone();
                    if let Some(refresh) = &token_response.refresh_token {
                        config.refresh_token = refresh.clone();
                    }
                    if let Some(expiry) = token_response.expires_in {
                        let expiry_time =
                            chrono::Utc::now() + chrono::Duration::seconds(expiry as i64);
                        config.token_expiry =
                            Some(expiry_time.format("%Y-%m-%dT%H:%M:%SZ").to_string());
                    }

                    let identifier = config.client_id.clone();
                    if let Err(e) = app.secret_store.store_oauth2_tokens(
                        &identifier,
                        &token_response.access_token,
                        token_response.refresh_token.as_deref().unwrap_or(""),
                        &config.client_secret,
                    ) {
                        log::warn!("Failed to store OAuth2 tokens in keyring: {e}");
                    }

                    app.toast_manager
                        .success("OAuth2 token received successfully".to_string());
                }
                Err(e) => {
                    app.toast_manager
                        .error(format!("OAuth2 token exchange failed: {e}"));
                }
            }
        }
    }
    Task::none()
}

pub fn handle_refresh_token(app: &mut AstraioApp, index: usize) -> Task<Message> {
    if let Some(view) = app.request_tabs.get(index) {
        if let Auth::OAuth2(config) = &view.auth {
            let http_client = app.http_client.clone();
            if !config.device_code.is_empty() {
                let token_url = config.token_url.clone();
                let device_code = config.device_code.clone();
                let client_id = config.client_id.clone();
                let client_secret = config.client_secret.clone();
                let tab_index = index;

                return Task::perform(
                    async move {
                        crate::data::oauth2::poll_device_token(
                            &http_client,
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
                            &http_client,
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

pub fn handle_start_device_auth(app: &AstraioApp, index: usize) -> Task<Message> {
    if let Some(view) = app.request_tabs.get(index) {
        if let Auth::OAuth2(config) = &view.auth {
            if config.device_auth_url.is_empty() {
                log::warn!("No device authorization URL configured");
            } else {
                let device_auth_url = config.device_auth_url.clone();
                let client_id = config.client_id.clone();
                let scopes = config.scopes.clone();
                let tab_index = index;
                let http_client = app.http_client.clone();

                return Task::perform(
                    async move {
                        crate::data::oauth2::device_authorization(
                            &http_client,
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
    app: &mut AstraioApp,
    index: usize,
    result: Result<crate::data::oauth2::DeviceAuthorizationResponse, AppError>,
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
                    config.auto_polling = true;

                    let verification_url = config.verification_uri.clone();
                    let user_code = config.user_code.clone();

                    log::info!("Device authorization received. User code: {user_code}");

                    app.toast_manager.info(format!(
                        "Device auth started. Code: {user_code}. Polling enabled."
                    ));

                    let _ = open::that(&verification_url);
                }
                Err(e) => {
                    log::error!("Device authorization failed: {e}");
                }
            }
        }
    }
    Task::none()
}

pub fn handle_device_token_poll(
    app: &mut AstraioApp,
    index: usize,
    result: Result<crate::data::oauth2::DeviceTokenResponse, AppError>,
) -> Task<Message> {
    if let Some(view) = app.request_tabs.get_mut(index) {
        if let Auth::OAuth2(config) = &mut view.auth {
            match result {
                Ok(device_token) => {
                    if let Some(access_token) = device_token.access_token {
                        config.access_token = access_token.clone();
                        if let Some(refresh) = device_token.refresh_token {
                            config.refresh_token = refresh;
                        }
                        config.device_code.clear();
                        config.user_code.clear();
                        config.verification_uri.clear();
                        config.auto_polling = false;

                        let identifier = config.client_id.clone();
                        if let Err(e) = app.secret_store.store_oauth2_tokens(
                            &identifier,
                            &access_token,
                            &config.refresh_token,
                            &config.client_secret,
                        ) {
                            log::warn!("Failed to store OAuth2 tokens in keyring: {e}");
                        }

                        log::info!("Device token received successfully");
                    } else if let Some(error) = device_token.error {
                        if error == "authorization_pending" {
                            log::info!("Authorization pending, polling again...");
                        } else if error == "slow_down" {
                            let current = config.device_code_interval.unwrap_or(5);
                            config.device_code_interval = Some(current + 5);
                            log::warn!(
                                "Slow down detected, increased interval to {}s",
                                current + 5
                            );
                        } else {
                            log::error!("Device token error: {error}");
                            config.device_code.clear();
                            config.user_code.clear();
                            config.verification_uri.clear();
                            config.auto_polling = false;
                        }
                    }
                }
                Err(e) => {
                    log::error!("Device token poll failed: {e}");
                    config.auto_polling = false;
                }
            }
        }
    }
    Task::none()
}

pub fn handle_auto_poll_toggle(app: &mut AstraioApp, index: usize, enabled: bool) -> Task<Message> {
    if let Some(view) = app.request_tabs.get_mut(index) {
        if let Auth::OAuth2(config) = &mut view.auth {
            config.auto_polling = enabled;
            if enabled {
                log::info!("Device code auto-polling enabled for tab {index}");
            } else {
                log::info!("Device code auto-polling disabled for tab {index}");
            }
        }
    }
    Task::none()
}

pub fn handle_graphql_start_auth(app: &mut AstraioApp) -> Task<Message> {
    if let Auth::OAuth2(config) = &mut app.graphql_view.auth {
        let pkce = if config.pkce_enabled {
            Some(crate::data::oauth2::PKCEChallenge::generate())
        } else {
            None
        };

        let state = crate::data::oauth2::generate_state();
        let pkce_verifier = pkce.as_ref().map(|p| p.verifier.clone());
        config.pkce_verifier = pkce_verifier.clone();

        if let Some(ref verifier) = pkce_verifier {
            let identifier = config.client_id.clone();
            if let Err(e) = app.secret_store.store_pkce_verifier(&identifier, verifier) {
                log::warn!("Failed to store PKCE verifier in keyring: {e}");
            }
        }

        let auth_url = config.auth_url.clone();
        let client_id = config.client_id.clone();
        let scopes = config.scopes.clone();

        return Task::perform(
            async move {
                let server = crate::data::oauth2::LocalAuthCallback::start().await?;
                let url = crate::data::oauth2::build_authorization_url(
                    &auth_url,
                    &client_id,
                    &server.redirect_uri,
                    &scopes,
                    pkce.as_ref(),
                    &state,
                );
                let _ = open::that(&url);
                let (code, _state) = server.wait_for_code(120).await?;
                Ok::<_, AppError>(code)
            },
            move |result| Message::GraphQLOAuth2AuthComplete(result, pkce_verifier),
        );
    }
    Task::none()
}

pub fn handle_graphql_auth_complete(
    app: &mut AstraioApp,
    result: Result<String, AppError>,
    pkce_verifier: Option<String>,
) -> Task<Message> {
    match result {
        Ok(code) => {
            if code.is_empty() {
                app.toast_manager
                    .warning("No authorization code received".to_string());
                return Task::none();
            }

            if let Auth::OAuth2(config) = &app.graphql_view.auth {
                let token_url = config.token_url.clone();
                let client_id = config.client_id.clone();
                let client_secret = config.client_secret.clone();
                let redirect_uri = config.redirect_uri.clone();
                let verifier = pkce_verifier.clone();
                let http_client = app.http_client.clone();

                return Task::perform(
                    async move {
                        crate::data::oauth2::exchange_code(
                            &http_client,
                            &token_url,
                            &code,
                            &client_id,
                            &client_secret,
                            &redirect_uri,
                            verifier.as_deref(),
                        )
                        .await
                    },
                    Message::GraphQLOAuth2TokenReceived,
                );
            }
            app.toast_manager
                .error("GraphQL auth not configured".to_string());
        }
        Err(e) => {
            app.toast_manager
                .error(format!("Authorization failed: {e}"));
        }
    }
    Task::none()
}

pub fn handle_graphql_token_received(
    app: &mut AstraioApp,
    result: Result<crate::data::oauth2::OAuth2TokenResponse, AppError>,
) -> Task<Message> {
    if let Auth::OAuth2(config) = &mut app.graphql_view.auth {
        match result {
            Ok(token_response) => {
                config.access_token = token_response.access_token.clone();
                if let Some(refresh) = &token_response.refresh_token {
                    config.refresh_token = refresh.clone();
                }
                if let Some(expiry) = token_response.expires_in {
                    let expiry_time = chrono::Utc::now() + chrono::Duration::seconds(expiry as i64);
                    config.token_expiry =
                        Some(expiry_time.format("%Y-%m-%dT%H:%M:%SZ").to_string());
                }

                let identifier = config.client_id.clone();
                if let Err(e) = app.secret_store.store_oauth2_tokens(
                    &identifier,
                    &token_response.access_token,
                    token_response.refresh_token.as_deref().unwrap_or(""),
                    &config.client_secret,
                ) {
                    log::warn!("Failed to store OAuth2 tokens in keyring: {e}");
                }

                app.toast_manager
                    .success("OAuth2 token received successfully".to_string());
            }
            Err(e) => {
                app.toast_manager
                    .error(format!("OAuth2 token exchange failed: {e}"));
            }
        }
    }
    Task::none()
}

pub fn handle_graphql_refresh_token(app: &mut AstraioApp) -> Task<Message> {
    if let Auth::OAuth2(config) = &app.graphql_view.auth {
        let http_client = app.http_client.clone();
        if !config.device_code.is_empty() {
            let token_url = config.token_url.clone();
            let device_code = config.device_code.clone();
            let client_id = config.client_id.clone();
            let client_secret = config.client_secret.clone();

            return Task::perform(
                async move {
                    crate::data::oauth2::poll_device_token(
                        &http_client,
                        &token_url,
                        &device_code,
                        &client_id,
                        &client_secret,
                    )
                    .await
                },
                Message::GraphQLOAuth2DeviceTokenPoll,
            );
        } else if config.refresh_token.is_empty() {
            app.toast_manager
                .warning("No refresh token available. Get a new token first.".to_string());
        } else {
            let token_url = config.token_url.clone();
            let refresh_token = config.refresh_token.clone();
            let client_id = config.client_id.clone();
            let client_secret = config.client_secret.clone();

            return Task::perform(
                async move {
                    crate::data::oauth2::refresh_token(
                        &http_client,
                        &token_url,
                        &refresh_token,
                        &client_id,
                        &client_secret,
                    )
                    .await
                },
                Message::GraphQLOAuth2TokenReceived,
            );
        }
    }
    Task::none()
}

pub fn handle_graphql_start_device_auth(app: &AstraioApp) -> Task<Message> {
    if let Auth::OAuth2(config) = &app.graphql_view.auth {
        if config.device_auth_url.is_empty() {
            log::warn!("No device authorization URL configured");
        } else {
            let device_auth_url = config.device_auth_url.clone();
            let client_id = config.client_id.clone();
            let scopes = config.scopes.clone();
            let http_client = app.http_client.clone();

            return Task::perform(
                async move {
                    crate::data::oauth2::device_authorization(
                        &http_client,
                        &device_auth_url,
                        &client_id,
                        &scopes,
                    )
                    .await
                },
                Message::GraphQLOAuth2DeviceAuthReceived,
            );
        }
    }
    Task::none()
}

pub fn handle_graphql_device_auth_received(
    app: &mut AstraioApp,
    result: Result<crate::data::oauth2::DeviceAuthorizationResponse, AppError>,
) -> Task<Message> {
    if let Auth::OAuth2(config) = &mut app.graphql_view.auth {
        match result {
            Ok(device_auth) => {
                config.device_code = device_auth.device_code;
                config.user_code = device_auth.user_code;
                config.verification_uri = device_auth.verification_uri;
                config.device_code_expires_in = Some(device_auth.expires_in);
                config.device_code_interval = device_auth.interval;
                config.auto_polling = true;

                let verification_url = config.verification_uri.clone();
                let user_code = config.user_code.clone();

                log::info!("Device authorization received. User code: {user_code}");

                app.toast_manager.info(format!(
                    "Device auth started. Code: {user_code}. Polling enabled."
                ));

                let _ = open::that(&verification_url);
            }
            Err(e) => {
                log::error!("Device authorization failed: {e}");
            }
        }
    }
    Task::none()
}

pub fn handle_graphql_device_token_poll(
    app: &mut AstraioApp,
    result: Result<crate::data::oauth2::DeviceTokenResponse, AppError>,
) -> Task<Message> {
    if let Auth::OAuth2(config) = &mut app.graphql_view.auth {
        match result {
            Ok(device_token) => {
                if let Some(access_token) = device_token.access_token {
                    config.access_token = access_token.clone();
                    if let Some(refresh) = device_token.refresh_token {
                        config.refresh_token = refresh;
                    }
                    config.device_code.clear();
                    config.user_code.clear();
                    config.verification_uri.clear();
                    config.auto_polling = false;

                    let identifier = config.client_id.clone();
                    if let Err(e) = app.secret_store.store_oauth2_tokens(
                        &identifier,
                        &access_token,
                        &config.refresh_token,
                        &config.client_secret,
                    ) {
                        log::warn!("Failed to store OAuth2 tokens in keyring: {e}");
                    }

                    log::info!("Device token received successfully");
                } else if let Some(error) = device_token.error {
                    if error == "authorization_pending" {
                        log::info!("Authorization pending, polling again...");
                    } else if error == "slow_down" {
                        let current = config.device_code_interval.unwrap_or(5);
                        config.device_code_interval = Some(current + 5);
                        log::warn!("Slow down detected, increased interval to {}s", current + 5);
                    } else {
                        log::error!("Device token error: {error}");
                        config.device_code.clear();
                        config.user_code.clear();
                        config.verification_uri.clear();
                        config.auto_polling = false;
                    }
                }
            }
            Err(e) => {
                log::error!("Device token poll failed: {e}");
                config.auto_polling = false;
            }
        }
    }
    Task::none()
}

pub fn handle_graphql_auto_poll_toggle(app: &mut AstraioApp, enabled: bool) -> Task<Message> {
    if let Auth::OAuth2(config) = &mut app.graphql_view.auth {
        config.auto_polling = enabled;
        if enabled {
            log::info!("GraphQL device code auto-polling enabled");
        } else {
            log::info!("GraphQL device code auto-polling disabled");
        }
    }
    Task::none()
}
