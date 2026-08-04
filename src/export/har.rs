#![allow(dead_code)]

use crate::http_client::config::RequestConfig;
use crate::http_client::request::HttpRequest;
use crate::http_client::response::HttpResponse;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct HarLog {
    pub version: String,
    pub creator: HarCreator,
    pub entries: Vec<HarEntry>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HarCreator {
    pub name: String,
    pub version: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HarEntry {
    #[serde(rename = "startedDateTime")]
    pub started_iso: String,
    pub time: u64,
    pub request: HarRequest,
    pub response: HarResponse,
    pub timings: HarTimings,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HarRequest {
    pub method: String,
    pub url: String,
    #[serde(rename = "httpVersion")]
    pub http_version: String,
    pub cookies: Vec<serde_json::Value>,
    pub headers: Vec<HarNameValue>,
    #[serde(rename = "queryString")]
    pub query_string: Vec<HarNameValuePair>,
    #[serde(rename = "postData")]
    pub post_data: Option<HarPostData>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HarResponse {
    pub status: u16,
    #[serde(rename = "statusText")]
    pub status_text: String,
    #[serde(rename = "httpVersion")]
    pub http_version: String,
    pub cookies: Vec<serde_json::Value>,
    pub headers: Vec<HarNameValue>,
    pub content: HarContent,
    #[serde(rename = "redirectUrl")]
    pub redirect_url: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HarContent {
    pub size: u64,
    #[serde(rename = "mimeType")]
    pub mime_type: String,
    pub text: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HarTimings {
    pub send: u64,
    pub wait: u64,
    pub receive: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HarNameValue {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HarNameValuePair {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HarPostData {
    pub mime_type: String,
    pub text: Option<String>,
}

fn now_iso() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}

fn parse_request_cookies(headers: &[(String, String)]) -> Vec<serde_json::Value> {
    let mut cookies = Vec::new();
    for (k, v) in headers {
        if k.eq_ignore_ascii_case("cookie") {
            for part in v.split(';') {
                let part = part.trim();
                if part.is_empty() {
                    continue;
                }
                let mut kv = part.splitn(2, '=');
                let name = kv.next().unwrap_or("").trim().to_string();
                let value = kv.next().unwrap_or("").trim().to_string();
                if !name.is_empty() {
                    cookies.push(serde_json::json!({
                        "name": name,
                        "value": value,
                    }));
                }
            }
        }
    }
    cookies
}

fn parse_response_cookies(headers: &[(String, String)]) -> Vec<serde_json::Value> {
    let mut cookies = Vec::new();
    for (k, v) in headers {
        if k.eq_ignore_ascii_case("set-cookie") {
            let mut parts = v.split(';');
            let first = match parts.next() {
                Some(p) => p.trim(),
                None => continue,
            };
            if first.is_empty() {
                continue;
            }
            let mut kv = first.splitn(2, '=');
            let name = kv.next().unwrap_or("").trim().to_string();
            let value = kv.next().unwrap_or("").trim().to_string();
            if name.is_empty() {
                continue;
            }

            let mut path: Option<String> = None;
            let mut domain: Option<String> = None;
            let mut expires: Option<String> = None;
            let mut http_only = false;
            let mut secure = false;

            for part in parts {
                let part = part.trim();
                if part.is_empty() {
                    continue;
                }
                let mut kv = part.splitn(2, '=');
                let attr_name = kv.next().unwrap_or("").trim().to_lowercase();
                let attr_val = kv.next().unwrap_or("").trim().to_string();

                match attr_name.as_str() {
                    "path" => path = Some(attr_val),
                    "domain" => domain = Some(attr_val),
                    "expires" => {
                        if let Ok(dt) = chrono::DateTime::parse_from_rfc2822(&attr_val) {
                            expires = Some(dt.to_rfc3339_opts(chrono::SecondsFormat::Millis, true));
                        } else {
                            expires = Some(attr_val);
                        }
                    }
                    "httponly" => http_only = true,
                    "secure" => secure = true,
                    _ => {}
                }
            }

            let mut cookie_obj = serde_json::json!({
                "name": name,
                "value": value,
            });

            if let Some(p) = path {
                cookie_obj["path"] = serde_json::Value::String(p);
            }
            if let Some(d) = domain {
                cookie_obj["domain"] = serde_json::Value::String(d);
            }
            if let Some(e) = expires {
                cookie_obj["expires"] = serde_json::Value::String(e);
            }
            if http_only {
                cookie_obj["httpOnly"] = serde_json::Value::Bool(true);
            }
            if secure {
                cookie_obj["secure"] = serde_json::Value::Bool(true);
            }

            cookies.push(cookie_obj);
        }
    }
    cookies
}

pub fn export_history_to_har(
    entries: &[crate::persistence::database::RequestHistoryEntry],
) -> String {
    let mut requests = Vec::new();
    let mut responses = Vec::new();
    for entry in entries {
        if let (Some(req_str), Some(resp_str)) = (&entry.request_data, &entry.response_data) {
            if let (Ok(req), Ok(resp)) = (
                serde_json::from_str::<HttpRequest>(req_str),
                serde_json::from_str::<HttpResponse>(resp_str),
            ) {
                requests.push(req);
                responses.push(resp);
            }
        }
    }
    let refs: Vec<(&HttpRequest, &HttpResponse)> = requests.iter().zip(responses.iter()).collect();
    export_entries(&refs)
}

pub fn export_collection_to_har(
    collection: &crate::persistence::database::Collection,
    requests: &[crate::persistence::database::CollectionRequest],
) -> String {
    let refs: Vec<(HttpRequest, HttpResponse)> = requests
        .iter()
        .map(|req| {
            let method: crate::http_client::request::HttpMethod =
                req.method
                    .parse()
                    .unwrap_or(crate::http_client::request::HttpMethod::Other(
                        req.method.clone(),
                    ));
            let url = if req.params.is_empty() {
                req.url.clone()
            } else {
                let query: String = req
                    .params
                    .iter()
                    .map(|(k, v)| format!("{}={}", urlencoding::encode(k), urlencoding::encode(v)))
                    .collect::<Vec<_>>()
                    .join("&");
                if req.url.contains('?') {
                    format!("{}&{}", req.url, query)
                } else {
                    format!("{}?{}", req.url, query)
                }
            };
            let headers = req.headers.clone();
            let body = req.body.clone();
            let _content_type = headers
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case("content-type"))
                .map_or_else(|| "application/json".to_string(), |(_, v)| v.clone());

            let http_req = HttpRequest {
                method,
                url: url.clone(),
                headers: headers.clone(),
                body: body.clone(),
                config: RequestConfig::default(),
                multipart_fields: Vec::new(),
                auth: None,
            };

            let http_resp = HttpResponse {
                url,
                method: http_req.method.clone(),
                status: 0,
                headers: Vec::new(),
                body: String::new(),
                body_encoding: crate::http_client::response::BodyEncoding::Text,
                duration: std::time::Duration::ZERO,
                size: 0,
                redirect_chain: Vec::new(),
            };

            (http_req, http_resp)
        })
        .collect();

    let refs: Vec<(&HttpRequest, &HttpResponse)> = refs.iter().map(|(r, s)| (r, s)).collect();
    let mut result = export_entries(&refs);

    // Inject collection name into creator for identification
    if let Ok(mut val) = serde_json::from_str::<serde_json::Value>(&result) {
        if let Some(log) = val.get_mut("log") {
            if let Some(creator) = log.get_mut("creator") {
                if let Some(name) = creator.get_mut("name") {
                    *name =
                        serde_json::Value::String(format!("Astraio Client - {}", collection.name));
                }
            }
        }
        result = serde_json::to_string_pretty(&val).unwrap_or(result);
    }

    result
}

pub fn export_entries(entries: &[(&HttpRequest, &HttpResponse)]) -> String {
    let har_entries: Vec<HarEntry> = entries
        .iter()
        .map(|(req, resp)| {
            let url = req.url.clone();
            let parsed_url = reqwest::Url::parse(&url);

            let query_string: Vec<HarNameValuePair> = parsed_url
                .as_ref()
                .ok()
                .map(|u| {
                    u.query_pairs()
                        .map(|(k, v)| HarNameValuePair {
                            name: k.to_string(),
                            value: v.to_string(),
                        })
                        .collect()
                })
                .unwrap_or_default();

            let headers: Vec<HarNameValue> = req
                .headers
                .iter()
                .map(|(k, v)| HarNameValue {
                    name: k.clone(),
                    value: v.clone(),
                })
                .collect();

            let resp_headers: Vec<HarNameValue> = resp
                .headers
                .iter()
                .map(|(k, v)| HarNameValue {
                    name: k.clone(),
                    value: v.clone(),
                })
                .collect();

            let content_type = resp
                .headers
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case("content-type"))
                .map_or_else(|| "text/plain".to_string(), |(_, v)| v.clone());

            let post_data = req.body.as_ref().map(|body| HarPostData {
                mime_type: req
                    .headers
                    .iter()
                    .find(|(k, _)| k.eq_ignore_ascii_case("content-type"))
                    .map_or_else(|| "application/json".to_string(), |(_, v)| v.clone()),
                text: Some(body.clone()),
            });

            let duration_ms = resp.duration.as_millis() as u64;

            HarEntry {
                started_iso: now_iso(),
                time: duration_ms,
                request: HarRequest {
                    method: req.method.to_string(),
                    url: url.clone(),
                    http_version: "HTTP/1.1".to_string(),
                    cookies: parse_request_cookies(&req.headers),
                    headers,
                    query_string,
                    post_data,
                },
                response: HarResponse {
                    status: resp.status,
                    status_text: status_text(resp.status),
                    http_version: "HTTP/1.1".to_string(),
                    cookies: parse_response_cookies(&resp.headers),
                    headers: resp_headers,
                    content: HarContent {
                        size: resp.size,
                        mime_type: content_type,
                        text: if resp.body_encoding
                            == crate::http_client::response::BodyEncoding::Text
                        {
                            Some(resp.body.clone())
                        } else {
                            None
                        },
                    },
                    redirect_url: String::new(),
                },
                timings: HarTimings {
                    send: 0,
                    wait: duration_ms,
                    receive: 0,
                },
            }
        })
        .collect();

    let har = HarLog {
        version: "1.2".to_string(),
        creator: HarCreator {
            name: "Astraio Client".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        },
        entries: har_entries,
    };

    let wrapper = serde_json::json!({ "log": har });
    serde_json::to_string_pretty(&wrapper).unwrap_or_else(|_| "{}".to_string())
}

fn status_text(status: u16) -> String {
    match status {
        200 => "OK".to_string(),
        201 => "Created".to_string(),
        204 => "No Content".to_string(),
        301 => "Moved Permanently".to_string(),
        302 => "Found".to_string(),
        304 => "Not Modified".to_string(),
        400 => "Bad Request".to_string(),
        401 => "Unauthorized".to_string(),
        403 => "Forbidden".to_string(),
        404 => "Not Found".to_string(),
        500 => "Internal Server Error".to_string(),
        502 => "Bad Gateway".to_string(),
        503 => "Service Unavailable".to_string(),
        _ => format!("Status {status}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::http_client::config::RequestConfig;
    use crate::http_client::request::HttpMethod;
    use crate::http_client::response::BodyEncoding;
    use std::time::Duration;

    #[test]
    fn export_single_entry() {
        let req = HttpRequest {
            method: HttpMethod::Get,
            url: "https://api.example.com/users?page=1".to_string(),
            headers: vec![
                ("Accept".to_string(), "application/json".to_string()),
                ("Authorization".to_string(), "Bearer token".to_string()),
            ],
            body: None,
            config: RequestConfig::default(),
            multipart_fields: vec![],
            auth: None,
        };
        let resp = HttpResponse {
            url: "https://api.example.com/users?page=1".to_string(),
            method: HttpMethod::Get,
            status: 200,
            headers: vec![
                ("content-type".to_string(), "application/json".to_string()),
                ("x-request-id".to_string(), "abc123".to_string()),
            ],
            body: r#"{"users": []}"#.to_string(),
            body_encoding: BodyEncoding::Text,
            duration: Duration::from_millis(150),
            size: 16,
            redirect_chain: vec![],
        };

        let har = export_entries(&[(&req, &resp)]);
        let parsed: serde_json::Value = serde_json::from_str(&har).unwrap();

        assert_eq!(parsed["log"]["version"], "1.2");
        assert_eq!(parsed["log"]["creator"]["name"], "Astraio Client");
        assert_eq!(parsed["log"]["entries"].as_array().unwrap().len(), 1);

        let entry = &parsed["log"]["entries"][0];
        assert_eq!(entry["request"]["method"], "GET");
        assert_eq!(
            entry["request"]["url"],
            "https://api.example.com/users?page=1"
        );
        assert_eq!(entry["response"]["status"], 200);
        assert_eq!(entry["response"]["content"]["text"], r#"{"users": []}"#);
        assert_eq!(entry["time"], 150);
    }

    #[test]
    fn export_post_with_body() {
        let req = HttpRequest {
            method: HttpMethod::Post,
            url: "https://api.example.com/users".to_string(),
            headers: vec![("Content-Type".to_string(), "application/json".to_string())],
            body: Some(r#"{"name": "John"}"#.to_string()),
            config: RequestConfig::default(),
            multipart_fields: vec![],
            auth: None,
        };
        let resp = HttpResponse {
            url: "https://api.example.com/users".to_string(),
            method: HttpMethod::Post,
            status: 201,
            headers: vec![],
            body: r#"{"id": 1}"#.to_string(),
            body_encoding: BodyEncoding::Text,
            duration: Duration::from_millis(200),
            size: 11,
            redirect_chain: vec![],
        };

        let har = export_entries(&[(&req, &resp)]);
        let parsed: serde_json::Value = serde_json::from_str(&har).unwrap();
        let entry = &parsed["log"]["entries"][0];

        assert_eq!(entry["request"]["method"], "POST");
        assert_eq!(entry["request"]["postData"]["text"], r#"{"name": "John"}"#);
        assert_eq!(entry["response"]["status"], 201);
    }

    #[test]
    fn export_multiple_entries() {
        let req1 = HttpRequest {
            method: HttpMethod::Get,
            url: "https://api.example.com/a".to_string(),
            headers: vec![],
            body: None,
            config: RequestConfig::default(),
            multipart_fields: vec![],
            auth: None,
        };
        let resp1 = HttpResponse {
            url: "https://api.example.com/a".to_string(),
            method: HttpMethod::Get,
            status: 200,
            headers: vec![],
            body: "ok".to_string(),
            body_encoding: BodyEncoding::Text,
            duration: Duration::from_millis(50),
            size: 2,
            redirect_chain: vec![],
        };
        let req2 = HttpRequest {
            method: HttpMethod::Delete,
            url: "https://api.example.com/b".to_string(),
            headers: vec![],
            body: None,
            config: RequestConfig::default(),
            multipart_fields: vec![],
            auth: None,
        };
        let resp2 = HttpResponse {
            url: "https://api.example.com/b".to_string(),
            method: HttpMethod::Delete,
            status: 204,
            headers: vec![],
            body: String::new(),
            body_encoding: BodyEncoding::Text,
            duration: Duration::from_millis(30),
            size: 0,
            redirect_chain: vec![],
        };

        let har = export_entries(&[(&req1, &resp1), (&req2, &resp2)]);
        let parsed: serde_json::Value = serde_json::from_str(&har).unwrap();
        assert_eq!(parsed["log"]["entries"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn status_text_map() {
        assert_eq!(status_text(200), "OK");
        assert_eq!(status_text(404), "Not Found");
        assert_eq!(status_text(500), "Internal Server Error");
        assert_eq!(status_text(999), "Status 999");
    }

    #[test]
    fn export_includes_query_params() {
        let req = HttpRequest {
            method: HttpMethod::Get,
            url: "https://api.example.com/search?q=rust&limit=10".to_string(),
            headers: vec![],
            body: None,
            config: RequestConfig::default(),
            multipart_fields: vec![],
            auth: None,
        };
        let resp = HttpResponse {
            url: "https://api.example.com/search?q=rust&limit=10".to_string(),
            method: HttpMethod::Get,
            status: 200,
            headers: vec![],
            body: "[]".to_string(),
            body_encoding: BodyEncoding::Text,
            duration: Duration::from_millis(100),
            size: 2,
            redirect_chain: vec![],
        };

        let har = export_entries(&[(&req, &resp)]);
        let parsed: serde_json::Value = serde_json::from_str(&har).unwrap();
        let qs = &parsed["log"]["entries"][0]["request"]["queryString"];
        assert_eq!(qs.as_array().unwrap().len(), 2);
    }

    #[test]
    fn test_cookie_parsing_in_har() {
        let req = HttpRequest {
            method: HttpMethod::Get,
            url: "https://api.example.com/search".to_string(),
            headers: vec![("Cookie".to_string(), "foo=bar; baz=qux".to_string())],
            body: None,
            config: RequestConfig::default(),
            multipart_fields: vec![],
            auth: None,
        };
        let resp = HttpResponse {
            url: "https://api.example.com/search".to_string(),
            method: HttpMethod::Get,
            status: 200,
            headers: vec![
                ("Set-Cookie".to_string(), "session=12345; Path=/; Domain=api.example.com; Expires=Sun, 19 Jul 2026 06:51:19 GMT; HttpOnly; Secure".to_string()),
            ],
            body: "[]".to_string(),
            body_encoding: BodyEncoding::Text,
            duration: Duration::from_millis(100),
            size: 2,
            redirect_chain: vec![],
        };

        let har = export_entries(&[(&req, &resp)]);
        let parsed: serde_json::Value = serde_json::from_str(&har).unwrap();

        let req_cookies = parsed["log"]["entries"][0]["request"]["cookies"]
            .as_array()
            .unwrap();
        assert_eq!(req_cookies.len(), 2);
        assert_eq!(req_cookies[0]["name"], "foo");
        assert_eq!(req_cookies[0]["value"], "bar");
        assert_eq!(req_cookies[1]["name"], "baz");
        assert_eq!(req_cookies[1]["value"], "qux");

        let resp_cookies = parsed["log"]["entries"][0]["response"]["cookies"]
            .as_array()
            .unwrap();
        assert_eq!(resp_cookies.len(), 1);
        assert_eq!(resp_cookies[0]["name"], "session");
        assert_eq!(resp_cookies[0]["value"], "12345");
        assert_eq!(resp_cookies[0]["path"], "/");
        assert_eq!(resp_cookies[0]["domain"], "api.example.com");
        assert_eq!(resp_cookies[0]["expires"], "2026-07-19T06:51:19.000Z");
        assert_eq!(resp_cookies[0]["httpOnly"], true);
        assert_eq!(resp_cookies[0]["secure"], true);
    }

    #[test]
    fn test_now_iso_format() {
        let iso = now_iso();
        assert!(iso.contains('T'));
        assert!(iso.ends_with('Z'));
        assert!(iso.len() >= 20);
    }
}
