use super::config::RequestConfig;
use super::request::{HttpRequest, MultipartValue};
use super::response::HttpResponse;
use std::time::{Duration, Instant};

pub fn build_client(config: &RequestConfig) -> Result<reqwest::Client, String> {
    let mut builder = reqwest::Client::builder();

    if let Some(proxy_url) = &config.proxy_url {
        let proxy = reqwest::Proxy::all(proxy_url).map_err(|e| e.to_string())?;
        builder = builder.proxy(proxy);
    }

    if !config.verify_ssl {
        builder = builder.danger_accept_invalid_certs(true);
    }

    builder
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|e| e.to_string())
}

pub async fn send_request(
    client: &reqwest::Client,
    request: HttpRequest,
) -> Result<HttpResponse, String> {
    let url_for_log = request.url.clone();
    let method_for_log = request.method.clone();
    let max_retries = request.config.retry.max_retries;
    let backoff_ms = request.config.retry.backoff_ms;
    let follow_redirects = request.config.follow_redirects;
    let max_redirects = request.config.max_redirects as usize;

    let mut last_error = String::new();

    for attempt in 0..=max_retries {
        if attempt > 0 {
            log::info!(
                "Retry attempt {}/{} after {}ms backoff",
                attempt,
                max_retries,
                backoff_ms
            );
            tokio::time::sleep(Duration::from_millis(backoff_ms * attempt as u64)).await;
        }

        let mut redirect_chain: Vec<String> = Vec::new();
        let mut current_url = request.url.clone();
        let mut response_status = 0u16;
        let mut response_headers = Vec::new();
        let mut response_body = String::new();
        let total_start = Instant::now();

        loop {
            let mut req_builder = client.request(
                request
                    .method
                    .parse()
                    .map_err(|e: http::method::InvalidMethod| e.to_string())?,
                current_url.clone(),
            );

            req_builder = req_builder.timeout(request.config.timeout);

            for (key, value) in &request.headers {
                req_builder = req_builder.header(key, value);
            }

            if !request.multipart_fields.is_empty() {
                let mut form = reqwest::multipart::Form::new();
                for field in &request.multipart_fields {
                    match &field.value {
                        MultipartValue::Text(text) => {
                            form = form.text(field.name.clone(), text.clone());
                        }
                        MultipartValue::File { path, filename } => {
                            let file_path = std::path::Path::new(path);
                            let file_name = filename
                                .as_deref()
                                .or_else(|| {
                                    file_path.file_name().map(|f| f.to_str().unwrap_or("file"))
                                })
                                .unwrap_or("file")
                                .to_string();

                            let file_bytes = match tokio::fs::read(file_path).await {
                                Ok(b) => b,
                                Err(e) => {
                                    last_error = format!("Failed to read file {}: {}", path, e);
                                    log::warn!("{}", last_error);
                                    continue;
                                }
                            };
                            let part =
                                reqwest::multipart::Part::bytes(file_bytes).file_name(file_name);
                            form = form.part(field.name.clone(), part);
                        }
                    }
                }
                req_builder = req_builder.multipart(form);
            } else if let Some(body) = &request.body {
                req_builder = req_builder.body(body.clone());
            }

            log::info!(
                "Sending {} request to: {} (attempt {}/{})",
                method_for_log,
                current_url,
                attempt + 1,
                max_retries + 1
            );

            match req_builder.send().await {
                Ok(res) => {
                    let status = res.status().as_u16();
                    let headers: Vec<(String, String)> = res
                        .headers()
                        .iter()
                        .map(|(name, value)| {
                            (name.to_string(), value.to_str().unwrap_or("").to_string())
                        })
                        .collect();

                    let is_redirect =
                        follow_redirects && (status == 301 || status == 302 || status == 303 || status == 307 || status == 308);

                    if is_redirect && redirect_chain.len() < max_redirects {
                        let location = res
                            .headers()
                            .get("location")
                            .and_then(|v| v.to_str().ok())
                            .unwrap_or("");

                        if location.is_empty() {
                            response_status = status;
                            response_headers = headers;
                            response_body = res.text().await.map_err(|e| e.to_string())?;
                            break;
                        }

                        redirect_chain.push(current_url.clone());
                        log::debug!("Redirect {} -> {}", status, location);

                        current_url = if location.starts_with("http") {
                            location.to_string()
                        } else {
                            let base = reqwest::Url::parse(&current_url).map_err(|e| e.to_string())?;
                            base.join(location).map_err(|e| e.to_string())?.to_string()
                        };
                        continue;
                    }

                    response_status = status;
                    response_headers = headers;
                    response_body = res.text().await.map_err(|e| e.to_string())?;
                    break;
                }
                Err(e) => {
                    last_error = e.to_string();
                    log::warn!(
                        "Request failed (attempt {}/{}): {}",
                        attempt + 1,
                        max_retries + 1,
                        last_error
                    );
                    break;
                }
            }
        }

        if last_error.is_empty() {
            let total_duration = total_start.elapsed();
            log::debug!("Total request completed in: {:?}", total_duration);

            let size = response_body.len() as u64;

            return Ok(HttpResponse {
                url: url_for_log,
                method: method_for_log,
                status: response_status,
                headers: response_headers,
                body: response_body,
                duration: total_duration,
                size,
                redirect_chain,
            });
        }

        if attempt == max_retries {
            return Err(last_error);
        }
    }

    Err(last_error)
}
