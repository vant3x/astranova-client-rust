use crate::http_client::config::RequestConfig;
use crate::http_client::request::HttpRequest;

#[derive(Debug, Clone)]
pub struct CurlParseResult {
    pub method: String,
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: Option<String>,
    pub config: RequestConfig,
}

pub fn parse_curl(curl: &str) -> Result<CurlParseResult, String> {
    let curl = curl.trim();
    let curl = if curl.starts_with("curl ") {
        &curl[5..]
    } else {
        curl
    };

    let tokens = tokenize(curl)?;
    let mut method: Option<String> = None;
    let mut url: Option<String> = None;
    let mut headers = Vec::new();
    let mut data: Option<String> = None;
    let mut config = RequestConfig::default();

    let mut i = 0;
    while i < tokens.len() {
        let token = &tokens[i];
        match token.as_str() {
            "-X" | "--request" => {
                i += 1;
                if i < tokens.len() {
                    method = Some(tokens[i].clone());
                }
            }
            "-H" | "--header" => {
                i += 1;
                if i < tokens.len() {
                    if let Some((key, value)) = parse_header(&tokens[i]) {
                        headers.push((key, value));
                    }
                }
            }
            "-d" | "--data" | "--data-raw" | "--data-binary" => {
                i += 1;
                if i < tokens.len() {
                    data = Some(tokens[i].clone());
                    if method.is_none() {
                        method = Some("POST".to_string());
                    }
                }
            }
            "-u" | "--user" => {
                i += 1;
                if i < tokens.len() {
                    if let Some((user, pass)) = parse_user_pass(&tokens[i]) {
                        let auth_header = format!("Basic {}", base64_encode(&format!("{}:{}", user, pass)));
                        headers.push(("Authorization".to_string(), auth_header));
                    }
                }
            }
            "--compressed" => {
                headers.push(("Accept-Encoding".to_string(), "gzip, deflate".to_string()));
            }
            "-k" | "--insecure" => {
                config.verify_ssl = false;
            }
            _ => {
                if token.starts_with("http://") || token.starts_with("https://") || token.starts_with("ws://") || token.starts_with("wss://") {
                    url = Some(token.clone());
                }
            }
        }
        i += 1;
    }

    let url = url.ok_or_else(|| "No URL found in curl command".to_string())?;
    let method = method.unwrap_or_else(|| {
        if data.is_some() {
            "POST".to_string()
        } else {
            "GET".to_string()
        }
    });

    Ok(CurlParseResult {
        method,
        url,
        headers,
        body: data,
        config: RequestConfig::default(),
    })
}

fn tokenize(input: &str) -> Result<Vec<String>, String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let mut escaped = false;

    for c in input.chars() {
        if escaped {
            current.push(c);
            escaped = false;
            continue;
        }
        if c == '\\' && !in_single_quote {
            escaped = true;
            continue;
        }
        if c == '\'' && !in_double_quote {
            in_single_quote = !in_single_quote;
            continue;
        }
        if c == '"' && !in_single_quote {
            in_double_quote = !in_double_quote;
            continue;
        }
        if c.is_whitespace() && !in_single_quote && !in_double_quote {
            if !current.is_empty() {
                tokens.push(current.clone());
                current.clear();
            }
            continue;
        }
        current.push(c);
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    Ok(tokens)
}

fn parse_header(header: &str) -> Option<(String, String)> {
    let colon_pos = header.find(':')?;
    let key = header[..colon_pos].trim().to_string();
    let value = header[colon_pos + 1..].trim().to_string();
    Some((key, value))
}

fn parse_user_pass(user_pass: &str) -> Option<(String, String)> {
    let colon_pos = user_pass.find(':')?;
    let user = user_pass[..colon_pos].to_string();
    let pass = user_pass[colon_pos + 1..].to_string();
    Some((user, pass))
}

fn base64_encode(input: &str) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(input)
}

pub fn curl_to_http_request(result: &CurlParseResult) -> HttpRequest {
    HttpRequest {
        method: result.method.clone(),
        url: result.url.clone(),
        headers: result.headers.clone(),
        body: result.body.clone(),
        config: result.config.clone(),
        multipart_fields: vec![],
        auth: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_get() {
        let curl = "curl https://api.example.com/users";
        let result = parse_curl(curl).unwrap();
        assert_eq!(result.method, "GET");
        assert_eq!(result.url, "https://api.example.com/users");
    }

    #[test]
    fn parse_post_with_data() {
        let curl = r#"curl -X POST https://api.example.com/users -d '{"name":"John"}'"#;
        let result = parse_curl(curl).unwrap();
        assert_eq!(result.method, "POST");
        assert_eq!(result.body, Some("{\"name\":\"John\"}".to_string()));
    }

    #[test]
    fn parse_with_headers() {
        let curl = r#"curl -H "Content-Type: application/json" -H "Authorization: Bearer token" https://api.example.com"#;
        let result = parse_curl(curl).unwrap();
        assert_eq!(result.headers.len(), 2);
        assert_eq!(result.headers[0].0, "Content-Type");
        assert_eq!(result.headers[0].1, "application/json");
    }

    #[test]
    fn parse_with_method() {
        let curl = "curl -X PUT https://api.example.com/users/1";
        let result = parse_curl(curl).unwrap();
        assert_eq!(result.method, "PUT");
    }

    #[test]
    fn parse_with_single_quotes() {
        let curl = "curl 'https://api.example.com/users'";
        let result = parse_curl(curl).unwrap();
        assert_eq!(result.url, "https://api.example.com/users");
    }

    #[test]
    fn parse_with_double_quotes() {
        let curl = r#"curl "https://api.example.com/users""#;
        let result = parse_curl(curl).unwrap();
        assert_eq!(result.url, "https://api.example.com/users");
    }

    #[test]
    fn parse_with_escaped_chars() {
        let curl = r#"curl https://api.example.com/users\?page=1"#;
        let result = parse_curl(curl).unwrap();
        assert_eq!(result.url, "https://api.example.com/users?page=1");
    }

    #[test]
    fn parse_with_basic_auth() {
        let curl = "curl -u user:pass https://api.example.com";
        let result = parse_curl(curl).unwrap();
        assert!(result.headers.iter().any(|(k, v)| k == "Authorization" && v.starts_with("Basic ")));
    }

    #[test]
    fn parse_with_compressed() {
        let curl = "curl --compressed https://api.example.com";
        let result = parse_curl(curl).unwrap();
        assert!(result.headers.iter().any(|(k, v)| k == "Accept-Encoding" && v == "gzip, deflate"));
    }

    #[test]
    fn parse_no_url_returns_error() {
        let curl = "curl";
        let result = parse_curl(curl);
        assert!(result.is_err());
    }

    #[test]
    fn convert_to_http_request() {
        let curl = "curl -X POST https://api.example.com -H \"Content-Type: application/json\" -d '{\"test\":true}'";
        let parsed = parse_curl(curl).unwrap();
        let request = curl_to_http_request(&parsed);
        assert_eq!(request.method, "POST");
        assert_eq!(request.url, "https://api.example.com");
        assert_eq!(request.headers.len(), 1);
        assert_eq!(request.body, Some("{\"test\":true}".to_string()));
    }
}
