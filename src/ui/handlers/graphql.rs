use crate::ui::app::{AstraioApp, Message};
use crate::ui::views::graphql_view;
use iced::Task;

pub fn handle_message(app: &mut AstraioApp, msg: graphql_view::Message) -> Task<Message> {
    match msg {
        graphql_view::Message::SendRequest => {
            let mut temp_view = app.graphql_view.clone_for_send();

            // Apply collection variables if available
            if let Some(env) = &app.active_environment {
                temp_view.apply_environment(env);
            }

            match temp_view.build_request() {
                Ok(_graphql_request) => {
                    let mut http_request = match temp_view.build_http_request() {
                        Ok(r) => r,
                        Err(e) => {
                            app.graphql_view
                                .update(graphql_view::Message::ResponseReceived(Err(e)));
                            return Task::none();
                        }
                    };
                    app.graphql_view.update(graphql_view::Message::SetLoading);

                    if let Ok(jar) = app.cookie_jar.lock() {
                        if let Some(cookie_header) = jar.to_cookie_header(&http_request.url) {
                            http_request
                                .headers
                                .retain(|(k, _)| !k.eq_ignore_ascii_case("cookie"));
                            http_request
                                .headers
                                .push(("cookie".to_string(), cookie_header));
                        }
                    } else {
                        log::error!(
                            "Failed to acquire cookie_jar lock for GraphQL cookie injection"
                        );
                    }

                    let needs_custom_client = http_request.config.proxy_url.is_some()
                        || http_request.config.proxy.is_some()
                        || !http_request.config.tls.verify_ssl
                        || http_request.config.tls.ca_cert_path.is_some()
                        || http_request.config.tls.client_cert_path.is_some()
                        || !http_request.config.cookie_store;

                    let http_client = if needs_custom_client {
                        let cache_key =
                            super::http_request::build_client_cache_key(&http_request.config);
                        if let Some((cached, last_used)) = app.custom_clients.get_mut(&cache_key) {
                            *last_used = std::time::Instant::now();
                            std::sync::Arc::clone(cached)
                        } else {
                            if app.custom_clients.len() >= 20 {
                                if let Some(oldest_key) = app
                                    .custom_clients
                                    .iter()
                                    .min_by_key(|(_, (_, t))| *t)
                                    .map(|(k, _)| k.clone())
                                {
                                    app.custom_clients.remove(&oldest_key);
                                }
                            }
                            match crate::http_client::client::build_client(&http_request.config) {
                                Ok(c) => {
                                    let c = std::sync::Arc::new(c);
                                    app.custom_clients.insert(
                                        cache_key,
                                        (std::sync::Arc::clone(&c), std::time::Instant::now()),
                                    );
                                    c
                                }
                                Err(e) => {
                                    log::error!("Failed to build custom client: {}", e);
                                    std::sync::Arc::clone(&app.http_client)
                                }
                            }
                        }
                    } else {
                        std::sync::Arc::clone(&app.http_client)
                    };

                    let request_url = http_request.url.clone();

                    Task::perform(
                        async move {
                            let response = crate::http_client::client::send_request(
                                &http_client,
                                http_request,
                            )
                            .await;

                            match response {
                                Ok(http_response) => {
                                    let graphql_response: crate::protocols::graphql::GraphQLResponse =
                                        serde_json::from_str(&http_response.body)
                                            .unwrap_or_else(|_| crate::protocols::graphql::GraphQLResponse {
                                                data: None,
                                                errors: vec![crate::protocols::graphql::GraphQLError {
                                                    message: format!(
                                                        "Failed to parse GraphQL response: {}",
                                                        http_response.body.chars().take(200).collect::<String>()
                                                    ),
                                                    locations: vec![],
                                                    path: vec![],
                                                    extensions: None,
                                                }],
                                            });

                                    Ok((
                                        graphql_response,
                                        http_response.status,
                                        http_response.headers,
                                        http_response.duration,
                                        http_response.size,
                                        request_url,
                                    ))
                                }
                                Err(e) => Err(e),
                            }
                        },
                        move |result| {
                            Message::GraphQLMsg(graphql_view::Message::ResponseReceived(result))
                        },
                    )
                }
                Err(e) => {
                    app.graphql_view
                        .update(graphql_view::Message::ResponseReceived(Err(e)));
                    Task::none()
                }
            }
        }
        graphql_view::Message::IntrospectSchema => {
            let mut temp_view = app.graphql_view.clone_for_send();
            if let Some(env) = &app.active_environment {
                temp_view.apply_environment(env);
            }
            let http_request = temp_view.build_introspection_request();

            let needs_custom_client = http_request.config.proxy_url.is_some()
                || http_request.config.proxy.is_some()
                || !http_request.config.tls.verify_ssl
                || http_request.config.tls.ca_cert_path.is_some()
                || http_request.config.tls.client_cert_path.is_some()
                || !http_request.config.cookie_store;

            let http_client = if needs_custom_client {
                let cache_key = super::http_request::build_client_cache_key(&http_request.config);
                if let Some((cached, last_used)) = app.custom_clients.get_mut(&cache_key) {
                    *last_used = std::time::Instant::now();
                    std::sync::Arc::clone(cached)
                } else {
                    match crate::http_client::client::build_client(&http_request.config) {
                        Ok(c) => {
                            let c = std::sync::Arc::new(c);
                            app.custom_clients.insert(
                                cache_key,
                                (std::sync::Arc::clone(&c), std::time::Instant::now()),
                            );
                            c
                        }
                        Err(e) => {
                            log::error!("Failed to build custom client: {}", e);
                            std::sync::Arc::clone(&app.http_client)
                        }
                    }
                }
            } else {
                std::sync::Arc::clone(&app.http_client)
            };

            Task::perform(
                async move {
                    let response =
                        crate::http_client::client::send_request(&http_client, http_request)
                            .await?;

                    let introspection: crate::protocols::graphql_schema::IntrospectionResponse =
                        serde_json::from_str(&response.body).map_err(|e| {
                            crate::error::AppError::Parse(format!(
                                "Failed to parse introspection response: {}",
                                e
                            ))
                        })?;

                    crate::protocols::graphql_schema::parse_introspection_response(&introspection)
                },
                move |result| Message::GraphQLMsg(graphql_view::Message::SchemaReceived(result)),
            )
        }
        graphql_view::Message::SaveToHistory => {
            let view = &app.graphql_view;
            let url = view.url_input.clone();
            let query = view.query_input.text();
            let variables = view.variables_input.text();
            let operation_name = view.operation_name.clone();

            let graphql_request = crate::protocols::graphql::GraphQLRequest {
                query: query.clone(),
                variables: if variables.trim().is_empty() {
                    None
                } else {
                    crate::protocols::graphql::parse_variables(&variables).ok()
                },
                operation_name: if operation_name.trim().is_empty() {
                    None
                } else {
                    Some(operation_name.clone())
                },
            };

            let request_data = serde_json::to_string(&graphql_request).ok();

            let response_data = view
                .last_response
                .as_ref()
                .and_then(|r| serde_json::to_string(r).ok());

            let result = crate::services::history_service::save_raw(
                &app.db_conn,
                "GRAPHQL",
                &url,
                view.status_code,
                view.response_duration.map(|d| d.as_millis() as u64),
                request_data.as_deref(),
                response_data.as_deref(),
            );

            match result {
                Ok(_) => {
                    app.graphql_view
                        .update(graphql_view::Message::SavedToHistory(Ok(())));
                    let _ = crate::services::history_service::trim(&app.db_conn, 500);
                    let entries = crate::services::history_service::get_all(&app.db_conn, 200)
                        .unwrap_or_default();
                    app.history_view.entries = entries;
                }
                Err(e) => {
                    app.graphql_view
                        .update(graphql_view::Message::SavedToHistory(Err(e)));
                }
            }
            Task::none()
        }
        graphql_view::Message::SaveToCollection(collection_id, folder_id) => {
            let view = &app.graphql_view;
            let url = view.url_input.clone();
            let query = view.query_input.text();
            let variables = view.variables_input.text();
            let operation_name = view.operation_name.clone();
            let headers: Vec<(String, String)> = view
                .headers_editor
                .entries
                .iter()
                .filter(|h| !h.key.is_empty())
                .map(|h| (h.key.clone(), h.value.clone()))
                .collect();

            let graphql_body = serde_json::to_string(&crate::protocols::graphql::GraphQLRequest {
                query,
                variables: if variables.trim().is_empty() {
                    None
                } else {
                    crate::protocols::graphql::parse_variables(&variables).ok()
                },
                operation_name: if operation_name.trim().is_empty() {
                    None
                } else {
                    Some(operation_name)
                },
            })
            .ok();

            let auth_json = view.auth.to_safe_json().ok();

            let resolved_collection_id = if collection_id == 0 {
                match crate::services::collection_service::get_all(&app.db_conn) {
                    Ok(cols) => {
                        if let Some(first) = cols.first() {
                            first.id
                        } else {
                            match crate::services::collection_service::create_and_refresh(
                                &app.db_conn,
                                "My Collection",
                            ) {
                                Ok(new_cols) => {
                                    if let Some(new_col) = new_cols.last() {
                                        app.collection_view.sync_collections(&new_cols);
                                        new_col.id
                                    } else {
                                        return Task::none();
                                    }
                                }
                                Err(e) => {
                                    log::error!("Failed to create collection: {}", e);
                                    return Task::none();
                                }
                            }
                        }
                    }
                    Err(e) => {
                        log::error!("Failed to load collections: {}", e);
                        return Task::none();
                    }
                }
            } else {
                collection_id
            };

            let result = crate::services::collection_service::save_request(
                &app.db_conn,
                &crate::persistence::database::SaveRequestParams {
                    collection_id: resolved_collection_id,
                    folder_id,
                    name: format!(
                        "GraphQL Request - {}",
                        url.chars().take(50).collect::<String>()
                    ),
                    method: "POST".to_string(),
                    url: url.clone(),
                    headers: headers.clone(),
                    body: graphql_body,
                    body_type: crate::persistence::database::CollectionBodyType::Graphql,
                    auth_type: crate::persistence::database::CollectionAuthType::None,
                    auth_data: auth_json,
                    params: Vec::new(),
                    config_json: None,
                    scripts: None,
                },
            );

            match result {
                Ok(_) => {
                    app.graphql_view
                        .update(graphql_view::Message::SavedToCollection(Ok(())));
                    let cols = crate::services::collection_service::get_all(&app.db_conn)
                        .unwrap_or_default();
                    app.collection_view.sync_collections(&cols);
                }
                Err(e) => {
                    app.graphql_view
                        .update(graphql_view::Message::SavedToCollection(Err(e)));
                }
            }
            Task::none()
        }
        graphql_view::Message::ResponseReceived(Ok(_)) => {
            if let graphql_view::Message::ResponseReceived(Ok((
                _,
                _,
                ref headers,
                _,
                _,
                ref request_url,
            ))) = msg
            {
                let mut captured_cookies = false;
                if let Ok(mut jar) = app.cookie_jar.lock() {
                    for (key, value) in headers {
                        if key.eq_ignore_ascii_case("set-cookie") {
                            jar.insert_from_set_cookie(value, request_url);
                            captured_cookies = true;
                        }
                    }
                    if captured_cookies {
                        if let Err(e) =
                            crate::persistence::database::save_cookies(&app.db_conn, &jar)
                        {
                            log::warn!("Failed to persist GraphQL cookies: {}", e);
                        }
                    }
                } else {
                    log::error!("Failed to acquire cookie_jar lock for GraphQL Set-Cookie capture");
                }
                if captured_cookies {
                    app.sync_cookie_data_to_tabs();
                }
            }
            app.graphql_view.update(msg);
            Task::none()
        }
        graphql_view::Message::OAuth2StartAuth => {
            Task::perform(async {}, |_| Message::GraphQLOAuth2StartAuth)
        }
        graphql_view::Message::OAuth2RefreshToken => {
            Task::perform(async {}, |_| Message::GraphQLOAuth2RefreshToken)
        }
        graphql_view::Message::OAuth2StartDeviceAuth => {
            Task::perform(async {}, |_| Message::GraphQLOAuth2StartDeviceAuth)
        }
        graphql_view::Message::OAuth2AutoPollToggle(enabled) => {
            Task::perform(async move {}, move |_| {
                Message::GraphQLOAuth2AutoPollToggle(enabled)
            })
        }
        other => {
            app.graphql_view.update(other);
            Task::none()
        }
    }
}
