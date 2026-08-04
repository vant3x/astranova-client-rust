use crate::http_client::client;
use crate::http_client::config::RequestConfig;
use crate::protocols::script_engine::ScriptEngineV2;
use crate::protocols::scripts::{ScriptContext, ScriptEngine};
use crate::ui::app::{AstraioApp, Message};
use crate::ui::views::http_request_view;
use iced::Task;
use std::sync::{Arc, Mutex};

pub(crate) fn build_client_cache_key(config: &RequestConfig) -> String {
    let proxy_part = match (&config.proxy_url, &config.proxy) {
        (Some(url), _) => url.clone(),
        (None, Some(proxy)) => {
            let user = proxy.auth.as_ref().map_or("", |a| a.username.as_str());
            format!("{}:{}", proxy.url, user)
        }
        (None, None) => String::new(),
    };
    format!(
        "{}|{}|{}|{}|{}|{}|{}",
        proxy_part,
        config.tls.verify_ssl,
        config.tls.ca_cert_path.as_deref().unwrap_or(""),
        config.tls.client_cert_path.as_deref().unwrap_or(""),
        config.tls.client_key_path.as_deref().unwrap_or(""),
        config.timeout.as_millis(),
        config.cookie_store,
    )
}

pub fn handle_http_request_msg(
    app: &mut AstraioApp,
    index: usize,
    msg: http_request_view::Message,
) -> Task<Message> {
    // Handle SessionLoad separately to avoid borrow conflicts with app.sync_cookie_data_to_tabs
    if let http_request_view::Message::SessionLoad(session_id) = &msg {
        let session_data = app
            .request_tabs
            .get(index)
            .and_then(|v| v.sessions.iter().find(|s| s.id == *session_id))
            .map(|s| {
                (
                    s.cookies_json.clone(),
                    s.headers_json.clone(),
                    s.auth_json.clone(),
                )
            });

        if let Some((cookies_json, headers_json, auth_json)) = session_data {
            // Load cookies into jar (app-level)
            if let Ok(jar) = crate::cookie::CookieJar::from_json(&cookies_json) {
                {
                    if let Ok(mut app_jar) = app.cookie_jar.lock() {
                        *app_jar = jar;
                    } else {
                        log::error!("Failed to acquire cookie_jar lock for session load");
                    }
                }
                if let Ok(jar) = app.cookie_jar.lock() {
                    if let Err(e) = crate::persistence::database::save_cookies(&app.db_conn, &jar) {
                        log::warn!("Failed to persist loaded cookies: {e}");
                    }
                }
                app.sync_cookie_data_to_tabs();
            }

            // Re-acquire view to load headers/auth
            if let Some(view) = app.request_tabs.get_mut(index) {
                if let Ok(headers) = serde_json::from_str::<Vec<serde_json::Value>>(&headers_json) {
                    view.headers_editor.entries.clear();
                    for (i, h) in headers.iter().enumerate() {
                        view.headers_editor.entries.push(
                            crate::ui::components::key_value_editor::KeyValueEntry {
                                id: i,
                                key: h["key"].as_str().unwrap_or("").to_string(),
                                value: h["value"].as_str().unwrap_or("").to_string(),
                                secret: h["secret"].as_bool().unwrap_or(false),
                            },
                        );
                    }
                }
                if let Some(auth_json) = &auth_json {
                    if let Ok(auth) = serde_json::from_str::<crate::data::auth::Auth>(auth_json) {
                        view.auth = auth;
                    }
                }
                view.selected_session = Some(session_id.clone());
            }
        }
        return Task::none();
    }

    let view = match app.request_tabs.get_mut(index) {
        Some(v) => v,
        None => return Task::none(),
    };

    match msg {
        http_request_view::Message::SendRequest => {
            let mut temp_view = view.clone_for_send();

            // Collect collection variables if a collection is selected
            let collection_vars: Vec<(String, String)> =
                if let Some(selected) = &app.collection_view.selected_item {
                    match selected {
                        crate::ui::views::collection_view::TreeItemId::Collection(idx) => app
                            .collection_view
                            .collections
                            .get(*idx)
                            .map(|c| c.variables.clone())
                            .unwrap_or_default(),
                        crate::ui::views::collection_view::TreeItemId::Folder(folder_id) => app
                            .collection_view
                            .folders
                            .iter()
                            .find(|f| f.id == *folder_id)
                            .and_then(|f| {
                                app.collection_view
                                    .collections
                                    .iter()
                                    .find(|c| c.id == f.collection_id)
                            })
                            .map(|c| c.variables.clone())
                            .unwrap_or_default(),
                        crate::ui::views::collection_view::TreeItemId::Request(req_id) => app
                            .collection_view
                            .requests
                            .iter()
                            .find(|r| r.id == *req_id)
                            .and_then(|r| {
                                app.collection_view
                                    .collections
                                    .iter()
                                    .find(|c| c.id == r.collection_id)
                            })
                            .map(|c| c.variables.clone())
                            .unwrap_or_default(),
                    }
                } else {
                    Vec::new()
                };

            if let Some(env) = &app.active_environment {
                // Merge: collection vars first, then environment vars override
                let merged = crate::utils::merge_variables(&collection_vars, &env.variables);
                let merged_env = crate::persistence::database::Environment {
                    id: env.id,
                    name: env.name.clone(),
                    variables: merged,
                    secret_keys: env.secret_keys.clone(),
                    default_endpoint: env.default_endpoint.clone(),
                };
                temp_view.apply_environment(&merged_env);
                let unresolved = temp_view.has_unresolved_variables();
                if !unresolved.is_empty() {
                    app.toast_manager
                        .warning(format!("Unresolved variables: {}", unresolved.join(", ")));
                }
            } else if !collection_vars.is_empty() {
                // Apply collection variables even without an environment
                let dummy_env = crate::persistence::database::Environment {
                    id: 0,
                    name: "collection".to_string(),
                    variables: collection_vars,
                    secret_keys: Vec::new(),
                    default_endpoint: None,
                };
                temp_view.apply_environment(&dummy_env);
            }

            // Validate URL before sending
            let url = temp_view.url_input.trim();
            if url.is_empty() {
                app.toast_manager.error("URL is required".to_string());
                return Task::none();
            }
            if reqwest::Url::parse(url).is_err() {
                app.toast_manager.error(format!("Invalid URL: {url}"));
                return Task::none();
            }

            // Auto-detect JSON body if content type is not explicitly set
            let body_text = temp_view.body_input.text();
            if !body_text.trim().is_empty()
                && temp_view.request_content_type
                    == crate::ui::views::http_request_view::ContentType::Text
            {
                let trimmed = body_text.trim();
                if ((trimmed.starts_with('{') && trimmed.ends_with('}'))
                    || (trimmed.starts_with('[') && trimmed.ends_with(']')))
                    && serde_json::from_str::<serde_json::Value>(trimmed).is_ok()
                {
                    temp_view.request_content_type =
                        crate::ui::views::http_request_view::ContentType::Json;
                }
            }

            let mut request = match temp_view.build_request() {
                Ok(r) => r,
                Err(e) => {
                    app.toast_manager
                        .error(format!("Failed to build request: {e}"));
                    return Task::none();
                }
            };

            let mut script_context = ScriptContext::new();
            if let Some(env) = &app.active_environment {
                for (key, value) in &env.variables {
                    script_context.variables.insert(key.clone(), value.clone());
                }
            }

            let pre_request_script = temp_view.scripts.pre_request.clone();
            let pre_request_js = temp_view.scripts.js_pre_request.clone();
            let mut delay_ms: Option<u64> = None;
            let mut script_output = http_request_view::ScriptOutput::default();

            // Detect if content is JS code or old JSON DSL
            let is_js_code = !pre_request_js.trim().is_empty()
                && serde_json::from_str::<crate::protocols::scripts::Script>(&pre_request_js)
                    .is_err();

            if is_js_code {
                // QuickJS engine path
                let mut variables = script_context.variables.clone();
                match ScriptEngineV2::execute_pre_request(
                    &pre_request_js,
                    &mut request,
                    &mut variables,
                ) {
                    Ok(output) => {
                        script_output.pre_logs = output.logs;
                        script_output.pre_errors = output.errors;
                        script_output.extracted_vars = output
                            .variables
                            .iter()
                            .map(|(k, v)| (k.clone(), v.clone()))
                            .collect();
                        script_output.test_results = output
                            .test_results
                            .iter()
                            .map(|t| http_request_view::TestResult {
                                name: t.name.clone(),
                                passed: t.passed,
                                message: t.message.clone(),
                            })
                            .collect();
                        script_context.variables = variables;
                    }
                    Err(e) => {
                        script_output.pre_errors.push(e.to_string());
                    }
                }
            } else if !pre_request_script.actions.is_empty() {
                // Legacy JSON DSL engine path
                for action in &pre_request_script.actions {
                    if let crate::protocols::scripts::ScriptAction::Delay { ms } = action {
                        delay_ms = Some(ms.saturating_add(delay_ms.unwrap_or(0)));
                    }
                }
                if let Err(e) = ScriptEngine::execute_pre_request(
                    &pre_request_script,
                    &mut request,
                    &mut script_context,
                ) {
                    script_output.pre_errors.push(e.to_string());
                }
                script_output.pre_logs.append(&mut script_context.logs);
                script_output.pre_errors.append(&mut script_context.errors);
            }

            for log in &script_output.pre_logs {
                app.toast_manager.info(format!("[Pre-request] {log}"));
            }

            let request_url = request.url.clone();
            let request_method = request.method.to_string();

            // Inject cookies from jar into request
            if let Ok(jar) = app.cookie_jar.lock() {
                if let Some(cookie_header) = jar.to_cookie_header(&request.url) {
                    request
                        .headers
                        .retain(|(k, _)| !k.eq_ignore_ascii_case("cookie"));
                    request.headers.push(("cookie".to_string(), cookie_header));
                }
            }

            view.pending_request_data = serde_json::to_string(&request).ok();
            view.update(http_request_view::Message::SetLoading);

            let needs_custom_client = request.config.proxy_url.is_some()
                || request.config.proxy.is_some()
                || !request.config.tls.verify_ssl
                || request.config.tls.ca_cert_path.is_some()
                || request.config.tls.client_cert_path.is_some()
                || !request.config.cookie_store;

            let http_client = if needs_custom_client {
                let cache_key = build_client_cache_key(&request.config);
                if let Some((cached, last_used)) = app.custom_clients.get_mut(&cache_key) {
                    *last_used = std::time::Instant::now();
                    Arc::clone(cached)
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
                    match client::build_client(&request.config) {
                        Ok(c) => {
                            let c = Arc::new(c);
                            app.custom_clients
                                .insert(cache_key, (Arc::clone(&c), std::time::Instant::now()));
                            c
                        }
                        Err(e) => {
                            log::error!("Failed to build custom client: {e}");
                            Arc::clone(&app.http_client)
                        }
                    }
                }
            } else {
                Arc::clone(&app.http_client)
            };

            let post_response_script = view.scripts.post_response.clone();
            let post_response_js = view.scripts.js_post_response.clone();
            let is_post_js = !post_response_js.trim().is_empty()
                && serde_json::from_str::<crate::protocols::scripts::Script>(&post_response_js)
                    .is_err();
            let has_scripts = is_post_js || !post_response_script.actions.is_empty();

            if has_scripts {
                // Buffered path: scripts need the full response body
                let (task, handle) = Task::perform(
                    async move {
                        if let Some(ms) = delay_ms {
                            tokio::time::sleep(std::time::Duration::from_millis(ms)).await;
                        }

                        let response = client::send_request(&http_client, request).await;

                        match response {
                            Ok(mut resp) => {
                                let mut warnings: Vec<String> = Vec::new();
                                let mut post_ctx = script_context;

                                if is_post_js {
                                    let mut variables = post_ctx.variables.clone();
                                    match ScriptEngineV2::execute_post_response(
                                        &post_response_js,
                                        &resp,
                                        &mut variables,
                                    ) {
                                        Ok(output) => {
                                            script_output.post_logs = output.logs;
                                            script_output.post_errors = output.errors;
                                            script_output.extracted_vars = output
                                                .variables
                                                .iter()
                                                .map(|(k, v)| (k.clone(), v.clone()))
                                                .collect();
                                            script_output.test_results = output
                                                .test_results
                                                .iter()
                                                .map(|t| http_request_view::TestResult {
                                                    name: t.name.clone(),
                                                    passed: t.passed,
                                                    message: t.message.clone(),
                                                })
                                                .collect();
                                        }
                                        Err(e) => {
                                            warnings
                                                .push(format!("Post-response script error: {e}"));
                                        }
                                    }
                                } else {
                                    if let Err(e) = ScriptEngine::execute_post_response(
                                        &post_response_script,
                                        &resp,
                                        &mut post_ctx,
                                    ) {
                                        warnings.push(format!("Post-response script error: {e}"));
                                    }
                                    script_output.post_logs.append(&mut post_ctx.logs);
                                    script_output.post_errors.append(&mut post_ctx.errors);
                                    for (k, v) in &post_ctx.variables {
                                        script_output.extracted_vars.push((k.clone(), v.clone()));
                                    }
                                }

                                for log in &script_output.post_logs {
                                    warnings.push(log.clone());
                                }
                                let output_json =
                                    serde_json::to_string(&script_output).unwrap_or_default();
                                warnings.push(format!("__SCRIPT_OUTPUT__{output_json}"));
                                resp.url = request_url;
                                resp.method = request_method
                                    .parse()
                                    .unwrap_or(crate::http_client::request::HttpMethod::Get);
                                (Ok(resp), warnings)
                            }
                            Err(e) => (Err(e), Vec::new()),
                        }
                    },
                    move |(result, warnings)| {
                        Message::HttpRequestViewMsg(
                            index,
                            http_request_view::Message::ResponseReceived(result, warnings),
                        )
                    },
                )
                .abortable();
                view.abort_handle = Some(handle);
                task
            } else {
                // Streaming path: UI stays responsive while body downloads
                let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
                let stream_id = app.http_stream_id;
                app.http_stream_id += 1;
                app.http_stream_receivers
                    .insert(index, (stream_id, Arc::new(Mutex::new(Some(rx)))));

                let http_client_clone = http_client;

                let _handle = tokio::spawn(async move {
                    if let Some(ms) = delay_ms {
                        tokio::time::sleep(std::time::Duration::from_millis(ms)).await;
                    }
                    if let Err(e) =
                        client::send_request_stream(&http_client_clone, request, tx.clone()).await
                    {
                        let _ =
                            tx.send(crate::http_client::response::HttpStreamEvent::StreamError(
                                e.to_string(),
                            ));
                    }
                });
                view.abort_handle = None;
                Task::none()
            }
        }
        http_request_view::Message::ResponseReceived(ref result, ref warnings) => {
            let Some(view) = app.request_tabs.get_mut(index) else {
                return Task::none();
            };
            for warning in warnings {
                if let Some(json) = warning.strip_prefix("__SCRIPT_OUTPUT__") {
                    if let Ok(output) =
                        serde_json::from_str::<http_request_view::ScriptOutput>(json)
                    {
                        view.script_output = output;
                    }
                } else {
                    app.toast_manager
                        .warning(format!("[Post-response] {warning}"));
                }
            }
            match result {
                Ok(response) => {
                    let request_data = view.pending_request_data.take();
                    let response_data = serde_json::to_string(response).ok().map(|data| {
                        let max = crate::persistence::database::MAX_HISTORY_RESPONSE_BYTES;
                        if data.len() > max {
                            let mut truncated: String = String::with_capacity(max + 64);
                            truncated.push_str(&data[..max]);
                            truncated.push_str("...\"__truncated__\":true}");
                            truncated
                        } else {
                            data
                        }
                    });

                    let (new_total, new_domains) = {
                        if let Ok(mut jar) = app.cookie_jar.lock() {
                            for (key, value) in &response.headers {
                                if key.eq_ignore_ascii_case("set-cookie") {
                                    jar.insert_from_set_cookie(value, &response.url);
                                }
                            }
                            let total = jar.total_count();
                            let domains = jar.domain_count();
                            if let Err(e) =
                                crate::persistence::database::save_cookies(&app.db_conn, &jar)
                            {
                                log::warn!("Failed to persist cookies: {e}");
                            }
                            (total, domains)
                        } else {
                            log::error!("Failed to acquire cookie_jar lock for response cookies");
                            (0, 0)
                        }
                    };
                    view.cookie_count = new_total;
                    view.cookie_domain_count = new_domains;

                    let history_result = crate::services::history_service::save_raw(
                        &app.db_conn,
                        &response.method.to_string(),
                        &response.url,
                        Some(response.status),
                        Some(response.duration.as_millis() as u64),
                        request_data.as_deref(),
                        response_data.as_deref(),
                    );
                    let _ = history_result;
                    let _ = crate::services::history_service::trim(
                        &app.db_conn,
                        crate::persistence::database::DEFAULT_HISTORY_LIMIT,
                    );
                    if app.show_history {
                        app.history_view.entries =
                            crate::services::history_service::get_all(&app.db_conn, 200)
                                .unwrap_or_default();
                    }

                    if response.status >= 400 {
                        app.toast_manager
                            .warning(format!("{} {}", response.status, response.url));
                    } else {
                        app.toast_manager.success(format!(
                            "{} {} - {}ms",
                            response.status,
                            response.url,
                            response.duration.as_millis()
                        ));
                    }
                }
                Err(e) => {
                    app.toast_manager.error(format!("Request failed: {e}"));
                }
            }
            if let Some(v) = app.request_tabs.get_mut(index) {
                v.update(msg);
            }
            app.sync_cookie_data_to_tabs();
            Task::none()
        }
        http_request_view::Message::MultipartBrowseFile(entry_id) => {
            let tab_index = index;
            Task::perform(
                async {
                    let file = rfd::AsyncFileDialog::new().pick_file().await;
                    file.map(|f| f.path().to_string_lossy().to_string())
                },
                move |path| {
                    Message::HttpRequestViewMsg(
                        tab_index,
                        http_request_view::Message::MultipartFilePicked(entry_id, path),
                    )
                },
            )
        }
        http_request_view::Message::OAuth2StartAuth => {
            Task::perform(async {}, move |()| Message::OAuth2StartAuth(index))
        }
        http_request_view::Message::OAuth2RefreshToken => {
            Task::perform(async {}, move |()| Message::OAuth2RefreshToken(index))
        }
        http_request_view::Message::OAuth2StartDeviceAuth => {
            Task::perform(async {}, move |()| Message::OAuth2StartDeviceAuth(index))
        }
        http_request_view::Message::OAuth2AutoPollToggle(enabled) => {
            Task::perform(async {}, move |()| {
                Message::OAuth2AutoPollToggle(index, enabled)
            })
        }
        http_request_view::Message::DownloadResponse => {
            let view = app.request_tabs.get(index).cloned();
            Task::perform(
                async move {
                    let view = match view {
                        Some(v) => v,
                        None => return Err("No tab".to_string()),
                    };
                    let response = match &view.last_response {
                        Some(r) => r.clone(),
                        None => return Err("No response".to_string()),
                    };
                    let content_type = response
                        .headers
                        .iter()
                        .find(|(k, _)| k.eq_ignore_ascii_case("content-type"))
                        .map_or_else(
                            || "application/octet-stream".to_string(),
                            |(_, v)| v.clone(),
                        );

                    let ext = if content_type.contains("image/png") {
                        "png"
                    } else if content_type.contains("image/jpeg")
                        || content_type.contains("image/jpg")
                    {
                        "jpg"
                    } else if content_type.contains("image/gif") {
                        "gif"
                    } else if content_type.contains("image/svg") {
                        "svg"
                    } else if content_type.contains("image/webp") {
                        "webp"
                    } else if content_type.contains("application/pdf") {
                        "pdf"
                    } else if content_type.contains("application/zip") {
                        "zip"
                    } else if content_type.contains("application/gzip")
                        || content_type.contains("application/x-gzip")
                    {
                        "gz"
                    } else if content_type.contains("application/json") {
                        "json"
                    } else if content_type.contains("application/xml")
                        || content_type.contains("text/xml")
                    {
                        "xml"
                    } else if content_type.contains("text/html") {
                        "html"
                    } else {
                        "bin"
                    };

                    let suggested_name = format!("response.{ext}");

                    let save_dialog = rfd::AsyncFileDialog::new()
                        .set_file_name(suggested_name)
                        .save_file()
                        .await;

                    let file_handle = match save_dialog {
                        Some(h) => h,
                        None => return Err("Cancelled".to_string()),
                    };

                    let bytes = if response.body_encoding
                        == crate::http_client::response::BodyEncoding::Base64
                    {
                        use base64::Engine;
                        base64::engine::general_purpose::STANDARD
                            .decode(&response.body)
                            .unwrap_or_default()
                    } else {
                        response.body.into_bytes()
                    };

                    let path = file_handle.path().to_path_buf();
                    tokio::fs::write(&path, &bytes)
                        .await
                        .map_err(|e| format!("Write error: {e}"))?;

                    Ok(path.to_string_lossy().to_string())
                },
                move |result| {
                    Message::HttpRequestViewMsg(
                        index,
                        http_request_view::Message::ResponseFileSaved(result),
                    )
                },
            )
        }
        http_request_view::Message::ResponseFileSaved(result) => {
            match result {
                Ok(path) => {
                    app.toast_manager.success(format!("Saved: {path}"));
                }
                Err(e) => {
                    if e != "Cancelled" {
                        app.toast_manager.error(e);
                    }
                }
            }
            Task::none()
        }
        http_request_view::Message::CancelRequest => {
            let Some(view) = app.request_tabs.get_mut(index) else {
                return Task::none();
            };
            if let Some(handle) = view.abort_handle.take() {
                handle.abort();
            }
            app.http_stream_receivers.remove(&index);
            view.update(http_request_view::Message::SetIdle);
            app.toast_manager.warning("Request cancelled".to_string());
            Task::none()
        }
        http_request_view::Message::SaveScripts => {
            let Some(view) = app.request_tabs.get_mut(index) else {
                return Task::none();
            };
            match view.parse_scripts_from_editors() {
                Ok(scripts) => {
                    view.scripts = scripts;
                    app.toast_manager.success("Scripts saved".to_string());
                    Task::perform(async move { Ok::<(), String>(()) }, move |_| {
                        Message::HttpRequestViewMsg(
                            index,
                            http_request_view::Message::ScriptsSaved(Ok(())),
                        )
                    })
                }
                Err(e) => {
                    app.toast_manager.error(format!("Invalid scripts: {e}"));
                    Task::none()
                }
            }
        }
        http_request_view::Message::ScriptsSaved(_) => Task::none(),
        http_request_view::Message::ClearKeychainSecrets => {
            if let Some(view) = app.request_tabs.get_mut(index) {
                view.update(http_request_view::Message::ClearKeychainSecrets);
            }
            Task::perform(async { Ok::<(), crate::error::AppError>(()) }, |_| {
                Message::ClearKeychainSecrets
            })
        }
        http_request_view::Message::ClearCookies => {
            Task::perform(async {}, |()| Message::ClearCookies)
        }
        http_request_view::Message::CookieManagerMsg(cm_msg) => {
            use crate::ui::views::cookie_manager::Message as CmMsg;
            match cm_msg {
                CmMsg::DeleteCookie(d, n, p) => {
                    Task::perform(async {}, move |()| Message::DeleteCookie(d, n, p))
                }
                CmMsg::ClearDomain(d) => {
                    Task::perform(async {}, move |()| Message::ClearDomainCookies(d))
                }
                CmMsg::ClearAll => Task::perform(async {}, |()| Message::ClearCookies),
                CmMsg::SaveEdit => {
                    let edit = app
                        .request_tabs
                        .get(index)
                        .and_then(|v| v.cookie_manager.editing_cookie.clone());
                    let value = app
                        .request_tabs
                        .get(index)
                        .map(|v| v.cookie_manager.edit_value.clone())
                        .unwrap_or_default();
                    if let Some((d, n, p)) = edit {
                        if let Some(view) = app.request_tabs.get_mut(index) {
                            view.update(http_request_view::Message::CookieManagerMsg(
                                CmMsg::SaveEdit,
                            ));
                        }
                        Task::perform(async {}, move |()| Message::SaveCookieEdit(d, n, p, value))
                    } else {
                        if let Some(view) = app.request_tabs.get_mut(index) {
                            view.update(http_request_view::Message::CookieManagerMsg(
                                CmMsg::SaveEdit,
                            ));
                        }
                        Task::none()
                    }
                }
                CmMsg::ImportCookies => Task::perform(async {}, |()| Message::ImportCookies),
                CmMsg::ExportCookies => Task::perform(async {}, |()| Message::ExportCookies),
                other => {
                    if let Some(view) = app.request_tabs.get_mut(index) {
                        view.update(http_request_view::Message::CookieManagerMsg(other));
                    }
                    Task::none()
                }
            }
        }
        http_request_view::Message::SessionSave(name) => {
            let name = name.trim().to_string();
            if name.is_empty() {
                return Task::none();
            }
            let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
            let cookies_json = app
                .cookie_jar
                .lock()
                .map_or_else(|_| Ok("{}".to_string()), |jar| jar.to_json_pretty())
                .unwrap_or_else(|_| "{}".to_string());
            let headers_json = serde_json::to_string(
                &view
                    .headers_editor
                    .entries
                    .iter()
                    .map(|e| {
                        serde_json::json!({
                            "key": e.key,
                            "value": e.value,
                            "secret": e.secret,
                        })
                    })
                    .collect::<Vec<_>>(),
            )
            .unwrap_or_else(|_| "[]".to_string());
            let auth_json = view.auth.to_safe_json().ok();
            let id = format!(
                "{}-{}",
                chrono::Utc::now().timestamp_millis(),
                &name[..name.len().min(8)]
            );
            let session = crate::persistence::database::Session {
                id: id.clone(),
                name: name.clone(),
                cookies_json,
                headers_json,
                auth_json,
                created_at: now.clone(),
                updated_at: now,
            };
            if let Err(e) = crate::persistence::database::save_session(&app.db_conn, &session) {
                app.toast_manager
                    .error(format!("Failed to save session: {e}"));
            } else {
                view.sessions.insert(0, session);
                view.new_session_name.clear();
                app.toast_manager.success(format!("Session saved: {name}"));
            }
            Task::none()
        }
        http_request_view::Message::SessionConfirmDelete(session_id) => {
            if let Err(e) = crate::persistence::database::delete_session(&app.db_conn, &session_id)
            {
                log::error!("Failed to delete session: {e}");
            }
            view.update(http_request_view::Message::SessionConfirmDelete(session_id));
            Task::none()
        }
        http_request_view::Message::SessionRenameConfirm => {
            if let Some(ref session_id) = view.renaming_session.clone() {
                let new_name = view.rename_value.trim().to_string();
                if !new_name.is_empty() {
                    if let Err(e) = crate::persistence::database::rename_session(
                        &app.db_conn,
                        session_id,
                        &new_name,
                    ) {
                        log::error!("Failed to rename session: {e}");
                    }
                }
            }
            view.update(http_request_view::Message::SessionRenameConfirm);
            Task::none()
        }
        other => {
            if let Some(view) = app.request_tabs.get_mut(index) {
                view.update(other);
            }
            Task::none()
        }
    }
}
