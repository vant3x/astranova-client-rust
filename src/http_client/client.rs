use super::config::RequestConfig;
use super::request::{HttpRequest, MultipartField, MultipartValue};
use super::response::{BodyEncoding, HttpStreamEvent, HttpResponse};
use crate::data::auth::Auth;
use crate::error::AppError;
use base64::Engine;
use std::collections::HashMap;
use std::time::{Duration, Instant};

const MAX_BODY_SIZE: usize = 10 * 1024 * 1024;

fn is_binary_content_type(content_type: &str) -> bool {
    content_type.contains("image/")
        || content_type.contains("application/octet-stream")
        || content_type.contains("application/pdf")
        || content_type.contains("application/protobuf")
        || content_type.contains("application/gzip")
        || content_type.contains("application/zip")
        || content_type.contains("application/wasm")
        || content_type.contains("audio/")
        || content_type.contains("video/")
        || content_type.contains("font/")
        || content_type.contains("application/x-executable")
        || content_type.contains("application/x-sharedlib")
}

async fn read_response_body(
    res: reqwest::Response,
) -> Result<(String, BodyEncoding, u64), AppError> {
    let content_type = res
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();

    if is_binary_content_type(&content_type) {
        let bytes = res.bytes().await?;
        let size = bytes.len() as u64;
        if bytes.len() > MAX_BODY_SIZE {
            let truncated = bytes.slice(..MAX_BODY_SIZE);
            let encoded = base64::engine::general_purpose::STANDARD.encode(&truncated);
            Ok((encoded, BodyEncoding::Base64, size))
        } else {
            let encoded = base64::engine::general_purpose::STANDARD.encode(&bytes);
            Ok((encoded, BodyEncoding::Base64, size))
        }
    } else {
        let text = res.text().await?;
        let size = text.len() as u64;
        if text.len() > MAX_BODY_SIZE {
            let truncated: String = text.chars().take(MAX_BODY_SIZE).collect();
            let display = format!(
                "{}\n\n--- Response truncated ({} bytes total, showing first {} bytes) ---",
                truncated, size, MAX_BODY_SIZE
            );
            Ok((display, BodyEncoding::Text, size))
        } else {
            Ok((text, BodyEncoding::Text, size))
        }
    }
}

pub fn build_client(config: &RequestConfig) -> Result<reqwest::Client, AppError> {
    let mut builder = reqwest::Client::builder();

    // User-Agent
    if !config.user_agent.is_empty() {
        builder = builder.user_agent(&config.user_agent);
    }

    // Cookie store
    if config.cookie_store {
        builder = builder.cookie_store(true);
    }

    // Proxy: prefer ProxyConfig (with auth) over flat proxy_url.
    // If no proxy is configured, reqwest automatically reads HTTP_PROXY/HTTPS_PROXY/NO_PROXY
    // from the environment.
    if let Some(proxy_config) = &config.proxy {
        let proxy = if let Some(auth) = &proxy_config.auth {
            let proxy_url = &proxy_config.url;
            let mut proxy = reqwest::Proxy::all(proxy_url)?;
            proxy = proxy.basic_auth(&auth.username, &auth.password);
            proxy
        } else {
            reqwest::Proxy::all(&proxy_config.url)?
        };
        builder = builder.proxy(proxy);
    } else if let Some(proxy_url) = &config.proxy_url {
        let proxy = reqwest::Proxy::all(proxy_url)?;
        builder = builder.proxy(proxy);
    }

    // TLS configuration
    let verify = config.tls.verify_ssl;
    if !verify {
        builder = builder.danger_accept_invalid_certs(true);
    }

    // CA certificate
    if let Some(ca_path) = &config.tls.ca_cert_path {
        let ca_cert = std::fs::read(ca_path)
            .map_err(|e| AppError::Http(format!("Failed to read CA cert {}: {}", ca_path, e)))?;
        let cert = reqwest::Certificate::from_pem(&ca_cert)
            .map_err(|e| AppError::Http(format!("Invalid CA cert: {}", e)))?;
        builder = builder.add_root_certificate(cert);
    }

    // Client certificate (mTLS)
    if let Some(cert_path) = &config.tls.client_cert_path {
        let key_path = config.tls.client_key_path.as_deref().ok_or_else(|| {
            AppError::Http("Client cert provided but no client key path".to_string())
        })?;

        let cert_bytes = std::fs::read(cert_path).map_err(|e| {
            AppError::Http(format!("Failed to read client cert {}: {}", cert_path, e))
        })?;
        let key_bytes = std::fs::read(key_path).map_err(|e| {
            AppError::Http(format!("Failed to read client key {}: {}", key_path, e))
        })?;

        // Combine cert + key PEM into a single identity
        let mut combined = cert_bytes;
        combined.extend_from_slice(&key_bytes);

        let identity = reqwest::Identity::from_pem(&combined)
            .map_err(|e| AppError::Http(format!("Invalid client certificate/key: {}", e)))?;
        builder = builder.identity(identity);
    }

    builder
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|e| AppError::Http(e.to_string()))
}

async fn build_multipart_form(
    fields: &[MultipartField],
) -> Result<reqwest::multipart::Form, AppError> {
    let mut form = reqwest::multipart::Form::new();
    for field in fields {
        match &field.value {
            MultipartValue::Text(text) => {
                form = form.text(field.name.clone(), text.clone());
            }
            MultipartValue::File { path, filename } => {
                let file_path = std::path::Path::new(path);
                let file_name = filename
                    .as_deref()
                    .or_else(|| file_path.file_name().map(|f| f.to_str().unwrap_or("file")))
                    .unwrap_or("file")
                    .to_string();

                let file = tokio::fs::File::open(file_path).await.map_err(|e| {
                    AppError::Http(format!(
                        "Failed to open file {}: {}",
                        file_path.display(),
                        e
                    ))
                })?;
                let stream = tokio_util::io::ReaderStream::new(file);
                let body = reqwest::Body::wrap_stream(stream);
                let part = reqwest::multipart::Part::stream(body)
                    .file_name(file_name)
                    .mime_str("application/octet-stream")
                    .unwrap_or_else(|_| {
                        reqwest::multipart::Part::bytes(vec![])
                            .file_name("file")
                    });
                form = form.part(field.name.clone(), part);
            }
        }
    }
    Ok(form)
}

/// Headers that must never be forwarded automatically to a different origin
/// when following a redirect (mirrors browser/reqwest-default behavior).
const SENSITIVE_REDIRECT_HEADERS: [&str; 3] = ["authorization", "cookie", "proxy-authorization"];

/// Returns true if `a` and `b` share scheme + host + (explicit or default) port.
fn same_origin(a: &str, b: &str) -> bool {
    match (reqwest::Url::parse(a), reqwest::Url::parse(b)) {
        (Ok(ua), Ok(ub)) => {
            ua.scheme() == ub.scheme()
                && ua.host_str() == ub.host_str()
                && ua.port_or_known_default() == ub.port_or_known_default()
        }
        // If either URL fails to parse, don't assume same-origin: fail closed.
        _ => false,
    }
}

async fn build_request_builder(
    client: &reqwest::Client,
    request: &HttpRequest,
    current_url: &str,
    original_url: &str,
) -> Result<reqwest::RequestBuilder, AppError> {
    let method: reqwest::Method = request.method.to_string().parse()?;
    let mut req_builder = client.request(method, current_url.to_string());

    req_builder = req_builder.timeout(request.config.timeout);

    // If a redirect took us to a different origin, drop credential-bearing
    // headers so they aren't leaked to a host the user didn't explicitly
    // target with them.
    let cross_origin = current_url != original_url && !same_origin(original_url, current_url);

    for (key, value) in &request.headers {
        if cross_origin && SENSITIVE_REDIRECT_HEADERS.contains(&key.to_lowercase().as_str()) {
            log::warn!(
                "Stripping '{}' header on cross-origin redirect ({} -> {})",
                key,
                original_url,
                current_url
            );
            continue;
        }
        req_builder = req_builder.header(key, value);
    }

    if !request.multipart_fields.is_empty() {
        let form = build_multipart_form(&request.multipart_fields).await?;
        req_builder = req_builder.multipart(form);
    } else if let Some(body) = &request.body {
        req_builder = req_builder.body(body.clone());
    }

    Ok(req_builder)
}

async fn attempt_digest_auth(
    client: &reqwest::Client,
    request: &HttpRequest,
    current_url: &str,
    www_auth: &str,
    user: &str,
    pass: &str,
) -> Result<Option<(u16, Vec<(String, String)>, String, BodyEncoding, u64)>, AppError> {
    if let Some(digest_header) = compute_digest_auth(
        www_auth,
        user,
        pass,
        &request.method.to_string(),
        current_url,
    ) {
        let mut retry_builder =
            client.request(request.method.to_string().parse()?, current_url.to_string());
        retry_builder = retry_builder.timeout(request.config.timeout);
        for (key, value) in &request.headers {
            retry_builder = retry_builder.header(key, value);
        }
        retry_builder = retry_builder.header("Authorization", digest_header);
        match retry_builder.send().await {
            Ok(retry_res) => {
                let response_status = retry_res.status().as_u16();
                let response_headers = retry_res
                    .headers()
                    .iter()
                    .map(|(name, value)| {
                        (name.to_string(), value.to_str().unwrap_or("").to_string())
                    })
                    .collect();
                let (body, encoding, size) = read_response_body(retry_res).await?;
                Ok(Some((
                    response_status,
                    response_headers,
                    body,
                    encoding,
                    size,
                )))
            }
            Err(e) => Err(AppError::from(e)),
        }
    } else {
        Ok(None)
    }
}

pub async fn send_request(
    client: &reqwest::Client,
    request: HttpRequest,
) -> Result<HttpResponse, AppError> {
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
        let mut response_body_encoding = BodyEncoding::Text;
        let mut response_size = 0u64;
        let total_start = Instant::now();

        loop {
            let req_builder =
                match build_request_builder(client, &request, &current_url, &request.url).await {
                    Ok(b) => b,
                    Err(e) => {
                        last_error = e.to_string();
                        break;
                    }
                };

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
                                    match attempt_digest_auth(
                                        client,
                                        &request,
                                        &current_url,
                                        www_auth,
                                        user,
                                        pass,
                                    )
                                    .await
                                    {
                                        Ok(Some((status, headers, body, encoding, size))) => {
                                            response_status = status;
                                            response_headers = headers;
                                            response_body = body;
                                            response_body_encoding = encoding;
                                            response_size = size;
                                            break;
                                        }
                                        Ok(None) => {}
                                        Err(e) => {
                                            last_error = e.to_string();
                                            break;
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
                            match read_response_body(res).await {
                                Ok((body, encoding, size)) => {
                                    response_body = body;
                                    response_body_encoding = encoding;
                                    response_size = size;
                                }
                                Err(e) => {
                                    last_error = e.to_string();
                                    break;
                                }
                            }
                            break;
                        }

                        redirect_chain.push(current_url.clone());
                        log::debug!("Redirect {} -> {}", status, location);

                        current_url = if location.starts_with("http://")
                            || location.starts_with("https://")
                        {
                            location.to_string()
                        } else {
                            match reqwest::Url::parse(&current_url) {
                                Ok(base) => match base.join(location) {
                                    Ok(joined) => joined.to_string(),
                                    Err(e) => {
                                        last_error = e.to_string();
                                        break;
                                    }
                                },
                                Err(e) => {
                                    last_error = e.to_string();
                                    break;
                                }
                            }
                        };
                        continue;
                    }

                    response_status = status;
                    response_headers = res_headers;
                    match read_response_body(res).await {
                        Ok((body, encoding, size)) => {
                            response_body = body;
                            response_body_encoding = encoding;
                            response_size = size;
                        }
                        Err(e) => {
                            last_error = e.to_string();
                            break;
                        }
                    }
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

            return Ok(HttpResponse {
                url: url_for_log,
                method: method_for_log,
                status: response_status,
                headers: response_headers,
                body: response_body,
                body_encoding: response_body_encoding,
                duration: total_duration,
                size: response_size,
                redirect_chain,
            });
        }

        if attempt == max_retries {
            return Err(AppError::Http(last_error));
        }
    }

    Err(AppError::Http(last_error))
}

/// Stream an HTTP response, sending events as headers and body chunks arrive.
/// This keeps the UI responsive for large responses.
pub async fn send_request_stream(
    client: &reqwest::Client,
    request: HttpRequest,
    tx: tokio::sync::mpsc::UnboundedSender<HttpStreamEvent>,
) -> Result<(), AppError> {
    let method_str = request.method.to_string();

    let req_builder =
        build_request_builder(client, &request, &request.url, &request.url).await?;

    log::info!("Streaming {} request to: {}", method_str, request.url);

    let res = req_builder.send().await.map_err(|e| {
        let _ = tx.send(HttpStreamEvent::StreamError(e.to_string()));
        AppError::from(e)
    })?;

    let status = res.status().as_u16();
    let headers: Vec<(String, String)> = res
        .headers()
        .iter()
        .map(|(name, value)| (name.to_string(), value.to_str().unwrap_or("").to_string()))
        .collect();

    let content_type = res
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();

    let binary = is_binary_content_type(&content_type);

    let _ = tx.send(HttpStreamEvent::HeadersReceived {
        status,
        headers,
        url: request.url.clone(),
        method: method_str.clone(),
    });

    let mut total_size: u64 = 0;
    let mut res = res;

    while let Some(chunk_result) = res.chunk().await.transpose() {
        match chunk_result {
            Ok(chunk) => {
                total_size += chunk.len() as u64;
                let event = if binary {
                    HttpStreamEvent::BodyChunkBinary(chunk.to_vec())
                } else {
                    HttpStreamEvent::BodyChunk(chunk.to_vec())
                };
                if tx.send(event).is_err() {
                    break;
                }
            }
            Err(e) => {
                let _ = tx.send(HttpStreamEvent::StreamError(e.to_string()));
                return Err(AppError::Http(e.to_string()));
            }
        }
    }

    let _ = tx.send(HttpStreamEvent::StreamComplete {
        total_size,
    });

    Ok(())
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
