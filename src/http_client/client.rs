use super::config::RequestConfig;
use super::request::{HttpRequest, MultipartValue};
use super::response::HttpResponse;
use crate::data::auth::Auth;
use std::collections::HashMap;
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
                    let res_headers: Vec<(String, String)> = res
                        .headers()
                        .iter()
                        .map(|(name, value)| {
                            (name.to_string(), value.to_str().unwrap_or("").to_string())
                        })
                        .collect();

                    if status == 401 {
                        if let Some(Auth::Digest { user, pass }) = &request.auth {
                            if let Some(www_auth) = res
                                .headers()
                                .get("www-authenticate")
                                .and_then(|v| v.to_str().ok())
                            {
                                if www_auth.starts_with("Digest ") {
                                    if let Some(digest_header) = compute_digest_auth(
                                        www_auth,
                                        user,
                                        pass,
                                        &request.method,
                                        &current_url,
                                    ) {
                                        let mut retry_builder = client.request(
                                            request.method.parse().map_err(
                                                |e: http::method::InvalidMethod| e.to_string(),
                                            )?,
                                            current_url.clone(),
                                        );
                                        retry_builder =
                                            retry_builder.timeout(request.config.timeout);
                                        for (key, value) in &request.headers {
                                            retry_builder = retry_builder.header(key, value);
                                        }
                                        retry_builder =
                                            retry_builder.header("Authorization", digest_header);
                                        match retry_builder.send().await {
                                            Ok(retry_res) => {
                                                response_status = retry_res.status().as_u16();
                                                response_headers = retry_res
                                                    .headers()
                                                    .iter()
                                                    .map(|(name, value)| {
                                                        (
                                                            name.to_string(),
                                                            value
                                                                .to_str()
                                                                .unwrap_or("")
                                                                .to_string(),
                                                        )
                                                    })
                                                    .collect();
                                                response_body = retry_res
                                                    .text()
                                                    .await
                                                    .map_err(|e| e.to_string())?;
                                                break;
                                            }
                                            Err(e) => {
                                                last_error = e.to_string();
                                                break;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    let is_redirect = matches!(
                        request.config.redirect_policy,
                        crate::http_client::config::RedirectPolicy::Follow
                            | crate::http_client::config::RedirectPolicy::Limited(_)
                    ) && (status == 301
                        || status == 302
                        || status == 303
                        || status == 307
                        || status == 308);

                    if is_redirect && redirect_chain.len() < max_redirects {
                        let location = res
                            .headers()
                            .get("location")
                            .and_then(|v| v.to_str().ok())
                            .unwrap_or("");

                        if location.is_empty() {
                            response_status = status;
                            response_headers = res_headers;
                            response_body = res.text().await.map_err(|e| e.to_string())?;
                            break;
                        }

                        redirect_chain.push(current_url.clone());
                        log::debug!("Redirect {} -> {}", status, location);

                        current_url = if location.starts_with("http") {
                            location.to_string()
                        } else {
                            let base =
                                reqwest::Url::parse(&current_url).map_err(|e| e.to_string())?;
                            base.join(location).map_err(|e| e.to_string())?.to_string()
                        };
                        continue;
                    }

                    response_status = status;
                    response_headers = res_headers;
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

fn compute_digest_auth(
    www_authenticate: &str,
    username: &str,
    password: &str,
    method: &str,
    url: &str,
) -> Option<String> {
    let params = parse_digest_params(www_authenticate);
    let realm = params.get("realm")?.clone();
    let nonce = params.get("nonce")?.clone();
    let qop = params
        .get("qop")
        .cloned()
        .unwrap_or_else(|| "auth".to_string());
    let opaque = params.get("opaque").cloned();
    let uri = url.split('?').next().unwrap_or("/").to_string();
    let nc = "00000001";
    let cnonce = format!(
        "{:x}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    );

    let ha1 = md5_hex(&format!("{}:{}:{}", username, realm, password));
    let ha2 = md5_hex(&format!("{}:{}", method, uri));
    let response_hash = md5_hex(&format!(
        "{}:{}:{}:{}:{}:{}",
        ha1, nonce, nc, cnonce, qop, ha2
    ));

    let mut parts = vec![
        format!("username=\"{}\"", username),
        format!("realm=\"{}\"", realm),
        format!("nonce=\"{}\"", nonce),
        format!("uri=\"{}\"", uri),
        format!("response=\"{}\"", response_hash),
        format!("qop={}", qop),
        format!("nc={}", nc),
        format!("cnonce=\"{}\"", cnonce),
    ];

    if let Some(opaque_val) = &opaque {
        parts.push(format!("opaque=\"{}\"", opaque_val));
    }

    Some(format!("Digest {}", parts.join(", ")))
}

fn parse_digest_params(header: &str) -> HashMap<String, String> {
    let mut params = HashMap::new();
    let header = header.strip_prefix("Digest ").unwrap_or(header);

    for part in header.split(',') {
        let part = part.trim();
        if let Some((key, value)) = part.split_once('=') {
            let key = key.trim().to_string();
            let value = value.trim().trim_matches('"').to_string();
            params.insert(key, value);
        }
    }
    params
}

fn md5_hex(input: &str) -> String {
    let hash = md5::compute(input.as_bytes());
    format!("{:x}", hash)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_digest_params_extracts_realm_and_nonce() {
        let header = r#"Digest realm="test@example.com", nonce="abc123", qop="auth""#;
        let params = parse_digest_params(header);
        assert_eq!(params.get("realm").unwrap(), "test@example.com");
        assert_eq!(params.get("nonce").unwrap(), "abc123");
        assert_eq!(params.get("qop").unwrap(), "auth");
    }

    #[test]
    fn parse_digest_params_extracts_opaque() {
        let header = r#"Digest realm="test", nonce="abc", opaque="xyz""#;
        let params = parse_digest_params(header);
        assert_eq!(params.get("opaque").unwrap(), "xyz");
    }

    #[test]
    fn md5_hex_produces_correct_hash() {
        let hash = md5_hex("hello");
        assert_eq!(hash, "5d41402abc4b2a76b9719d911017c592");
    }

    #[test]
    fn compute_digest_auth_returns_none_without_realm() {
        let result = compute_digest_auth(
            "Bearer token",
            "user",
            "pass",
            "GET",
            "https://example.com/",
        );
        assert!(result.is_none());
    }

    #[test]
    fn compute_digest_auth_returns_header_with_valid_input() {
        let header = r#"Digest realm="test@example.com", nonce="nonce123", qop="auth""#;
        let result =
            compute_digest_auth(header, "admin", "secret", "GET", "https://example.com/api");
        assert!(result.is_some());
        let auth_header = result.unwrap();
        assert!(auth_header.starts_with("Digest "));
        assert!(auth_header.contains("username=\"admin\""));
        assert!(auth_header.contains("realm=\"test@example.com\""));
        assert!(auth_header.contains("nonce=\"nonce123\""));
        assert!(auth_header.contains("qop=auth"));
    }
}
