use super::request::HttpRequest;
use super::response::HttpResponse;

pub async fn send_request(request: HttpRequest) -> Result<HttpResponse, String> {
    let client = reqwest::Client::new();

    let url_for_log = request.url.clone();
    let method_for_log = request.method.clone();
    let request_body_for_log = request.body.clone();

    let mut req_builder = client.request(request.method.parse().map_err(|e: http::method::InvalidMethod| e.to_string())?, request.url);

    for (key, value) in request.headers {
        req_builder = req_builder.header(&key, &value);
    }

    if let Some(body) = request_body_for_log.clone() { // Use cloned body for req_builder
        req_builder = req_builder.body(body);
    }

    println!("[HTTP_CLIENT] Sending request to: {}", url_for_log);
    let start_time = std::time::Instant::now();

    let res = req_builder.send().await.map_err(|e| e.to_string())?;

    let network_duration = start_time.elapsed();
    println!("[HTTP_CLIENT] Network request completed in: {:?}", network_duration);

    let status = res.status().as_u16();
    let headers = res.headers().iter()
        .map(|(name, value)| (name.to_string(), value.to_str().unwrap_or("").to_string()))
        .collect();
    let body_start_time = std::time::Instant::now();
    let body = res.text().await.map_err(|e| e.to_string())?;
    let body_read_duration = body_start_time.elapsed();
    println!("[HTTP_CLIENT] Body read completed in: {:?}", body_read_duration);

    let size = body.len() as u64;

    Ok(HttpResponse {
        url: url_for_log,
        method: method_for_log,
        status,
        headers,
        body,
        duration: network_duration,
        size,
    })
}
