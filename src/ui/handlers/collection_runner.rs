use crate::http_client::config::RequestConfig;
use crate::http_client::request::{HttpMethod, HttpRequest};
use crate::persistence::database::CollectionRequest;
use crate::ui::app::{AstraioApp, Message};
use crate::ui::views::collection_runner::{self, CollectionRunnerState, RequestRunResult};
use iced::Task;

pub fn handle_message(app: &mut AstraioApp, msg: collection_runner::Message) -> Task<Message> {
    match msg.clone() {
        collection_runner::Message::StartRun(col_id, col_name, requests) => {
            app.collection_runner_state =
                Some(CollectionRunnerState::new(col_id, col_name, requests));
            app.show_collection_runner = true;

            let runner = app.collection_runner_state.clone().unwrap();
            let env = app.active_environment.clone();
            let collection_vars = runner
                .requests
                .first()
                .map(|_r| {
                    app.collection_view
                        .collections
                        .iter()
                        .find(|c| c.id == runner.collection_id)
                        .map(|c| c.variables.clone())
                        .unwrap_or_default()
                })
                .unwrap_or_default();

            return Task::perform(
                run_collection_async(runner, env, collection_vars),
                |results| {
                    Message::CollectionRunnerMsg(collection_runner::Message::RunCompleted(results))
                },
            );
        }
        collection_runner::Message::RequestCompleted(result) => {
            if let Some(runner) = &mut app.collection_runner_state {
                if result.passed {
                    runner.passed += 1;
                } else {
                    runner.failed += 1;
                }
                runner.results.push(result);
                runner.current_index += 1;
            }
        }
        collection_runner::Message::ToggleStopOnFailure(enabled) => {
            if let Some(runner) = &mut app.collection_runner_state {
                runner.stop_on_failure = enabled;
            }
        }
        collection_runner::Message::DelayChanged(delay) => {
            if let Some(runner) = &mut app.collection_runner_state {
                runner.delay_ms = delay;
            }
        }
        collection_runner::Message::Stop => {
            if let Some(runner) = &mut app.collection_runner_state {
                runner.is_cancelled = true;
                runner.is_running = false;
            }
        }
        collection_runner::Message::Close => {
            app.show_collection_runner = false;
            app.collection_runner_state = None;
        }
        collection_runner::Message::RunCompleted(results) => {
            if let Some(runner) = &mut app.collection_runner_state {
                runner.results = results;
                runner.passed = runner.results.iter().filter(|r| r.passed).count();
                runner.failed = runner.results.iter().filter(|r| !r.passed).count();
                runner.is_running = false;
                runner.current_index = runner.results.len();
            }
        }
    }
    Task::none()
}

async fn run_collection_async(
    runner: CollectionRunnerState,
    active_environment: Option<crate::persistence::database::Environment>,
    collection_variables: Vec<(String, String)>,
) -> Vec<RequestRunResult> {
    let mut results = Vec::new();
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());

    let delay_ms: u64 = runner.delay_ms.parse().unwrap_or(0);

    for (i, entry) in runner.requests.iter().enumerate() {
        if runner.is_cancelled {
            break;
        }

        let mut request = build_request_from_entry(entry);

        // Apply environment variables
        if let Some(env) = &active_environment {
            apply_variables(&mut request, &env.variables);
        }

        // Apply collection variables (override env)
        if !collection_variables.is_empty() {
            apply_variables(&mut request, &collection_variables);
        }

        let start = std::time::Instant::now();
        let result = execute_single(&client, &request).await;
        let duration = start.elapsed().as_millis() as u64;

        let run_result = RequestRunResult {
            request_id: entry.id,
            name: entry.name.clone(),
            method: entry.method.clone(),
            url: entry.url.clone(),
            status_code: result.status,
            duration_ms: duration,
            passed: result.success,
            error: result.error,
        };

        results.push(run_result);

        // Check stop on failure
        if runner.stop_on_failure && !results.last().is_none_or(|r| r.passed) {
            break;
        }

        // Delay between requests
        if delay_ms > 0 && i < runner.requests.len() - 1 {
            tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
        }
    }

    results
}

fn build_request_from_entry(entry: &CollectionRequest) -> HttpRequest {
    let mut headers = entry.headers.clone();

    if entry.body.is_some()
        && entry.body_type == crate::persistence::database::CollectionBodyType::Text
    {
        let content_type = if let Some(body) = &entry.body {
            if body.trim_start().starts_with('{') || body.trim_start().starts_with('[') {
                "application/json"
            } else if body.trim_start().starts_with('<') {
                "application/xml"
            } else {
                "text/plain"
            }
        } else {
            "text/plain"
        };
        headers.push(("Content-Type".to_string(), content_type.to_string()));
    }

    let method = entry.method.parse().unwrap_or(HttpMethod::Get);

    HttpRequest {
        method,
        url: entry.url.clone(),
        headers,
        body: entry.body.clone(),
        config: RequestConfig::default(),
        multipart_fields: Vec::new(),
        auth: None,
    }
}

fn apply_variables(request: &mut HttpRequest, variables: &[(String, String)]) {
    for (key, value) in variables {
        let placeholder = format!("{{{{{key}}}}}");
        request.url = request.url.replace(placeholder.as_str(), value);
        for (_, v) in &mut request.headers {
            *v = v.replace(placeholder.as_str(), value);
        }
        if let Some(body) = &mut request.body {
            *body = body.replace(placeholder.as_str(), value);
        }
    }
}

struct SingleResult {
    status: Option<u16>,
    success: bool,
    error: Option<String>,
}

async fn execute_single(client: &reqwest::Client, request: &HttpRequest) -> SingleResult {
    let method = match request.method {
        HttpMethod::Get => reqwest::Method::GET,
        HttpMethod::Post => reqwest::Method::POST,
        HttpMethod::Put => reqwest::Method::PUT,
        HttpMethod::Delete => reqwest::Method::DELETE,
        HttpMethod::Patch => reqwest::Method::PATCH,
        HttpMethod::Head => reqwest::Method::HEAD,
        HttpMethod::Options => reqwest::Method::OPTIONS,
        _ => reqwest::Method::GET,
    };

    let mut req_builder = client.request(method, &request.url);

    for (key, value) in &request.headers {
        req_builder = req_builder.header(key, value);
    }

    if let Some(body) = &request.body {
        req_builder = req_builder.body(body.clone());
    }

    match req_builder.send().await {
        Ok(response) => {
            let status = response.status().as_u16();
            SingleResult {
                status: Some(status),
                success: (200..300).contains(&status),
                error: None,
            }
        }
        Err(e) => SingleResult {
            status: None,
            success: false,
            error: Some(e.to_string()),
        },
    }
}
