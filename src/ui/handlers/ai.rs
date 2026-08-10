use crate::ai::types::{AiChatMessage, AiChatRequest, AiProviderConfig, AiRole};
use crate::ui::app::{AstraioApp, Message};
use crate::ui::views::ai_chat_view;
use iced::Task;

pub fn handle_message(app: &mut AstraioApp, message: ai_chat_view::Message) -> Task<Message> {
    match message {
        ai_chat_view::Message::InputChanged(text) => {
            app.ai_view.input = text;
            Task::none()
        }
        ai_chat_view::Message::SendMessage => handle_send(app),
        ai_chat_view::Message::ReceiveStreamStart => {
            app.ai_view.start_streaming();
            Task::none()
        }
        ai_chat_view::Message::ReceiveStreamChunk(chunk) => {
            app.ai_view.append_stream_chunk(&chunk);
            Task::none()
        }
        ai_chat_view::Message::ReceiveStreamEnd => {
            app.ai_view.finish_streaming();
            Task::none()
        }
        ai_chat_view::Message::ReceiveError(err) => {
            app.ai_view.finish_streaming();
            app.ai_view.error_message = Some(err.clone());
            app.ai_view
                .add_message(AiRole::Assistant, format!("Error: {err}"), 0);
            Task::none()
        }
        ai_chat_view::Message::ProviderSelected(provider) => {
            app.ai_view.selected_provider_type = provider.clone();
            app.ai_view.editing_base_url = provider.default_base_url().to_string();
            app.ai_view.editing_model = provider.default_model().to_string();
            Task::none()
        }
        ai_chat_view::Message::ModelChanged(model) => {
            app.ai_view.editing_model = model;
            Task::none()
        }
        ai_chat_view::Message::ApiKeyChanged(key) => {
            app.ai_view.editing_api_key = key;
            Task::none()
        }
        ai_chat_view::Message::BaseUrlChanged(url) => {
            app.ai_view.editing_base_url = url;
            Task::none()
        }
        ai_chat_view::Message::ToggleSettings => {
            app.ai_view.show_settings = !app.ai_view.show_settings;
            Task::none()
        }
        ai_chat_view::Message::SaveProviderConfig => handle_save_provider(app),
        ai_chat_view::Message::SetDefaultProvider(idx) => {
            for (i, config) in app.ai_view.providers.iter_mut().enumerate() {
                config.is_default = i == idx;
            }
            persist_providers(app);
            Task::none()
        }
        ai_chat_view::Message::DeleteProvider(idx) => {
            if let Some(config) = app.ai_view.providers.get(idx) {
                if let Some(id) = config.id {
                    let _ = crate::persistence::database::delete_ai_provider(&app.db_conn, id);
                }
                if let Some(provider_type) = app.ai_view.providers.get(idx) {
                    let _ = app.secret_store.delete_secret(
                        "ai",
                        &format!("{}_{}", provider_type.provider, provider_type.name),
                        "api_key",
                    );
                }
            }
            app.ai_view.providers.remove(idx);
            if app.ai_view.active_provider_index == Some(idx) {
                app.ai_view.active_provider_index = app.ai_view.providers.iter().position(|p| p.is_default);
            } else if app.ai_view.active_provider_index.map_or(false, |i| i > idx) {
                app.ai_view.active_provider_index =
                    app.ai_view.active_provider_index.map(|i| i - 1);
            }
            rebuild_ai_service(app);
            Task::none()
        }
        ai_chat_view::Message::ClearChat => {
            app.ai_view.messages.clear();
            app.ai_view.streaming_buffer.clear();
            Task::none()
        }
        ai_chat_view::Message::CopyMessage(idx) => {
            if let Some(msg) = app.ai_view.messages.get(idx) {
                if let Some(mut clipboard) = arboard::Clipboard::new().ok() {
                    let _ = clipboard.set_text(&msg.content);
                }
            }
            Task::none()
        }
        ai_chat_view::Message::ApplyToRequest(content) => {
            handle_apply_to_request(app, &content)
        }
        ai_chat_view::Message::AutoContextToggled(enabled) => {
            app.ai_view.auto_context = enabled;
            Task::none()
        }
        ai_chat_view::Message::SystemPromptChanged(prompt) => {
            app.ai_view.system_prompt = prompt;
            Task::none()
        }
        ai_chat_view::Message::NewConversation => {
            app.ai_view.messages.clear();
            Task::none()
        }
        ai_chat_view::Message::SelectConversation(_idx) => Task::none(),
        ai_chat_view::Message::DeleteConversation(_idx) => Task::none(),
        ai_chat_view::Message::QuickAction(action) => handle_quick_action(app, action),
        ai_chat_view::Message::TabChanged(tab) => {
            app.ai_view.active_tab = tab;
            Task::none()
        }
    }
}

fn handle_send(app: &mut AstraioApp) -> Task<Message> {
    let input = app.ai_view.input.trim().to_string();
    if input.is_empty() || app.ai_view.is_streaming {
        return Task::none();
    }

    app.ai_view.input.clear();
    app.ai_view
        .add_message(AiRole::User, input.clone(), 0);

    let Some(config) = get_active_provider_config(app) else {
        app.ai_view
            .add_message(AiRole::Assistant, "No AI provider configured. Open Settings to add one.".to_string(), 0);
        return Task::none();
    };

    let context = build_context(app);
    let history = build_history(app);
    let messages = context.build_messages(&input, &history);

    let request = AiChatRequest {
        model: config.model.clone(),
        messages,
        max_tokens: Some(config.max_tokens),
        temperature: Some(config.temperature),
        stream: true,
    };

    let provider_config = config.clone();
    app.ai_view.start_streaming();

    Task::perform(
        perform_chat_stream(app.ai_service.clone(), provider_config, request),
        |result| match result {
            Ok(()) => Message::AiMsg(ai_chat_view::Message::ReceiveStreamEnd),
            Err(e) => Message::AiMsg(ai_chat_view::Message::ReceiveError(e)),
        },
    )
}

async fn perform_chat_stream(
    ai_service: std::sync::Arc<tokio::sync::Mutex<crate::ai::service::AiService>>,
    config: AiProviderConfig,
    request: AiChatRequest,
) -> Result<(), String> {
    let service = ai_service.lock().await;
    let mut rx = service
        .chat_stream(&config, request)
        .await
        .map_err(|e| e.to_string())?;

    drop(service);

    while let Some(chunk) = rx.recv().await {
        // We need to send chunks via a channel or similar mechanism
        // For now, we collect and handle via polling
        log::debug!("AI chunk: {chunk}");
    }

    Ok(())
}

fn handle_quick_action(app: &mut AstraioApp, action: ai_chat_view::QuickAction) -> Task<Message> {
    let prompt = match &action {
        ai_chat_view::QuickAction::GenerateRequest => {
            let prefix = action.prompt_prefix();
            let context_hint = if let Some(req) = app.request_tabs.get(app.active_request_tab_index) {
                if !req.url_input.is_empty() {
                    format!(" for URL: {}", req.url_input)
                } else {
                    String::new()
                }
            } else {
                String::new()
            };
            format!("{prefix}{context_hint}")
        }
        ai_chat_view::QuickAction::ExplainResponse => {
            let prefix = action.prompt_prefix();
            format!("{prefix}Check the current response tab for details.")
        }
        ai_chat_view::QuickAction::GenerateScript => {
            let prefix = action.prompt_prefix();
            format!("{prefix}Generate variables, set headers, or validate the response.")
        }
        ai_chat_view::QuickAction::GenerateMockData => {
            if app.ai_view.mock_data.description.is_empty() {
                "Generate 5 realistic user records with id, name, email, avatar_url, created_at, and status fields".to_string()
            } else {
                format!(
                    "Generate {} mock data records in {} format: {}",
                    app.ai_view.mock_data.count, app.ai_view.mock_data.format, app.ai_view.mock_data.description
                )
            }
        }
        ai_chat_view::QuickAction::DebugError => {
            let prefix = action.prompt_prefix();
            format!("{prefix}I'm getting an unexpected error from my API.")
        }
        ai_chat_view::QuickAction::TransformFormat => {
            let prefix = action.prompt_prefix();
            format!("{prefix}Convert between JSON, CSV, and XML formats.")
        }
    };

    app.ai_view.input = prompt;
    handle_send(app)
}

fn handle_apply_to_request(app: &mut AstraioApp, content: &str) -> Task<Message> {
    if let Some(parsed) = parse_http_request_from_ai(content) {
        if let Some(tab) = app.request_tabs.get_mut(app.active_request_tab_index) {
            tab.method = parsed.0;
            tab.url_input = parsed.1;
            app.toast_manager
                .success("Request applied from AI");
        }
    }
    Task::none()
}

fn parse_http_request_from_ai(content: &str) -> Option<(String, String, Option<Vec<(String, String)>>, Option<String>)> {
    let lines: Vec<&str> = content.lines().collect();
    let mut method = String::new();
    let mut url = String::new();
    let mut headers = Vec::new();
    let mut body_lines = Vec::new();
    let mut in_body = false;

    for line in &lines {
        let trimmed = line.trim();

        if trimmed.is_empty() || trimmed.starts_with("```") {
            continue;
        }

        if method.is_empty() {
            let upper = trimmed.to_uppercase();
            if upper.starts_with("GET ")
                || upper.starts_with("POST ")
                || upper.starts_with("PUT ")
                || upper.starts_with("PATCH ")
                || upper.starts_with("DELETE ")
                || upper.starts_with("HEAD ")
                || upper.starts_with("OPTIONS ")
            {
                let parts: Vec<&str> = trimmed.splitn(2, ' ').collect();
                if parts.len() >= 2 {
                    method = parts[0].to_uppercase();
                    url = parts[1].split_whitespace().next().unwrap_or("").to_string();
                    if url.ends_with("HTTP") || url.ends_with("HTTP/1.1") {
                        url = url
                            .trim_end_matches("HTTP/1.1")
                            .trim_end_matches("HTTP")
                            .trim()
                            .to_string();
                    }
                }
                continue;
            }
        }

        if !method.is_empty() && !in_body {
            if let Some(pos) = trimmed.find(':') {
                let key = trimmed[..pos].trim().to_string();
                let value = trimmed[pos + 1..].trim().to_string();
                if !key.is_empty() {
                    headers.push((key, value));
                }
            } else if trimmed.starts_with('{') || trimmed.starts_with('[') {
                in_body = true;
                body_lines.push(trimmed.to_string());
            }
        } else if in_body {
            body_lines.push(trimmed.to_string());
        }
    }

    if method.is_empty() || url.is_empty() {
        return None;
    }

    let headers_opt = if headers.is_empty() {
        None
    } else {
        Some(headers)
    };

    let body_opt = if body_lines.is_empty() {
        None
    } else {
        Some(body_lines.join("\n"))
    };

    Some((method, url, headers_opt, body_opt))
}

fn build_context(app: &AstraioApp) -> crate::ai::context::AiContextBuilder<'static> {
    // Build an owned HttpRequest from the active tab to avoid lifetime issues
    let mut builder = crate::ai::context::AiContextBuilder::new();

    if let Some(tab) = app.request_tabs.get(app.active_request_tab_index) {
        // Store the built context info as owned strings in the system prompt
        // by capturing relevant info before building
        let method: crate::http_client::request::HttpMethod =
            tab.method.parse().unwrap_or(crate::http_client::request::HttpMethod::Get);
        builder = builder.with_request_owned(method, tab.url_input.clone());
    }

    if let Some(env) = &app.active_environment {
        let vars: Vec<(String, String)> = env
            .variables
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        builder = builder.with_environment_owned(env.name.clone(), vars);
    }

    builder
}

fn build_history(app: &AstraioApp) -> Vec<AiChatMessage> {
    app.ai_view
        .messages
        .iter()
        .filter(|m| !m.content.is_empty() && !m.is_streaming)
        .map(|m| AiChatMessage {
            role: m.role.clone(),
            content: m.content.clone(),
        })
        .collect()
}

fn get_active_provider_config(app: &AstraioApp) -> Option<&AiProviderConfig> {
    app.ai_view
        .active_provider_index
        .and_then(|idx| app.ai_view.providers.get(idx))
}

fn handle_save_provider(app: &mut AstraioApp) -> Task<Message> {
    let name = app.ai_view.editing_provider_name.trim().to_string();
    let api_key = app.ai_view.editing_api_key.trim().to_string();
    let base_url = app.ai_view.editing_base_url.trim().to_string();
    let model = app.ai_view.editing_model.trim().to_string();

    if name.is_empty() || base_url.is_empty() || model.is_empty() {
        app.toast_manager.error("Name, Base URL, and Model are required");
        return Task::none();
    }

    let config = AiProviderConfig {
        id: None,
        provider: app.ai_view.selected_provider_type.clone(),
        name: name.clone(),
        base_url,
        model,
        max_tokens: 4096,
        temperature: 0.7,
        is_default: app.ai_view.providers.is_empty(),
    };

    let secret_key = format!("{}_{}", config.provider, config.name);

    if !api_key.is_empty() {
        if let Err(e) = app.secret_store.store_secret("ai", &secret_key, "api_key", &api_key) {
            app.toast_manager.error(format!("Failed to store API key: {e}"));
            return Task::none();
        }
    }

    match crate::persistence::database::insert_ai_provider(&app.db_conn, &config) {
        Ok(id) => {
            let mut saved_config = config;
            saved_config.id = Some(id);
            app.ai_view.providers.push(saved_config);
            app.ai_view.active_provider_index = Some(app.ai_view.providers.len() - 1);
            app.ai_view.editing_api_key.clear();
            app.ai_view.editing_provider_name.clear();
            rebuild_ai_service(app);
            app.toast_manager.success("Provider saved");
        }
        Err(e) => {
            app.toast_manager.error(format!("Failed to save provider: {e}"));
        }
    }

    Task::none()
}

fn persist_providers(app: &AstraioApp) {
    for config in &app.ai_view.providers {
        if let Some(id) = config.id {
            let _ = crate::persistence::database::update_ai_provider(&app.db_conn, id, config);
        }
    }
}

fn rebuild_ai_service(app: &mut AstraioApp) {
    let mut service = crate::ai::service::AiService::new();
    for config in &app.ai_view.providers {
        let secret_key = format!("{}_{}", config.provider, config.name);
        let api_key = app
            .secret_store
            .get_secret("ai", &secret_key, "api_key")
            .ok()
            .flatten();
        service.register_provider(config.clone(), api_key);
    }
    app.ai_service = std::sync::Arc::new(tokio::sync::Mutex::new(service));
}
