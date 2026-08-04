use crate::error::AppError;

#[derive(Debug, Clone)]
pub struct CurlParseResult {
    pub method: String,
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: Option<String>,
    pub auth_user: Option<String>,
    pub auth_pass: Option<String>,
    pub form_fields: Vec<(String, String)>,
    pub insecure: bool,
}

pub fn parse_curl(curl: &str) -> Result<CurlParseResult, AppError> {
    let curl = curl.trim();
    let curl = curl.strip_prefix("curl ").unwrap_or(curl);

    let tokens = tokenize(curl)?;
    let mut method: Option<String> = None;
    let mut url: Option<String> = None;
    let mut headers = Vec::new();
    let mut data: Option<String> = None;
    let mut auth_user: Option<String> = None;
    let mut auth_pass: Option<String> = None;
    let mut form_fields = Vec::new();
    let mut insecure = false;

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
                        if key.eq_ignore_ascii_case("Accept-Encoding") {
                            headers.retain(|(k, _): &(String, String)| {
                                !k.eq_ignore_ascii_case("Accept-Encoding")
                            });
                        }
                        headers.push((key, value));
                    }
                }
            }
            "-d" | "--data" | "--data-raw" | "--data-binary" => {
                i += 1;
                if i < tokens.len() {
                    let value = if tokens[i].starts_with('@') {
                        let file_path = &tokens[i][1..];
                        match std::fs::read_to_string(file_path) {
                            Ok(content) => content,
                            Err(_) => tokens[i].clone(),
                        }
                    } else {
                        tokens[i].clone()
                    };
                    data = Some(value);
                    if method.is_none() {
                        method = Some("POST".to_string());
                    }
                }
            }
            "--data-urlencode" => {
                i += 1;
                if i < tokens.len() {
                    let encoded = if tokens[i].contains('=') {
                        let eq_pos = tokens[i].find('=').unwrap();
                        let key = &tokens[i][..eq_pos];
                        let value = &tokens[i][eq_pos + 1..];
                        let encoded_value = if let Some(file_path) = value.strip_prefix('@') {
                            match std::fs::read_to_string(file_path) {
                                Ok(content) => {
                                    let trimmed = content.trim().to_string();
                                    urlencoding::encode(&trimmed).to_string()
                                }
                                Err(_) => urlencoding::encode(value).to_string(),
                            }
                        } else {
                            urlencoding::encode(value).to_string()
                        };
                        format!("{key}={encoded_value}")
                    } else if tokens[i].starts_with('@') {
                        let file_path = &tokens[i][1..];
                        match std::fs::read_to_string(file_path) {
                            Ok(content) => {
                                let trimmed = content.trim().to_string();
                                urlencoding::encode(&trimmed).to_string()
                            }
                            Err(_) => tokens[i].clone(),
                        }
                    } else {
                        urlencoding::encode(&tokens[i]).to_string()
                    };
                    data = Some(encoded);
                    if method.is_none() {
                        method = Some("POST".to_string());
                    }
                }
            }
            "-F" | "--form" => {
                i += 1;
                if i < tokens.len() {
                    if let Some((key, value)) = parse_form_field(&tokens[i]) {
                        form_fields.push((key, value));
                    }
                    if method.is_none() {
                        method = Some("POST".to_string());
                    }
                }
            }
            "-u" | "--user" => {
                i += 1;
                if i < tokens.len() {
                    if let Some((user, pass)) = parse_user_pass(&tokens[i]) {
                        auth_user = Some(user);
                        auth_pass = Some(pass);
                    }
                }
            }
            "--compressed" => {
                if !headers
                    .iter()
                    .any(|(k, _)| k.eq_ignore_ascii_case("Accept-Encoding"))
                {
                    headers.push(("Accept-Encoding".to_string(), "gzip, deflate".to_string()));
                }
            }
            "-k" | "--insecure" => {
                insecure = true;
            }
            _ => {
                if token.contains("://") || (!token.starts_with('-') && token.contains('/')) {
                    url = Some(token.clone());
                }
            }
        }
        i += 1;
    }

    let url = url.ok_or_else(|| AppError::Parse("No URL found in curl command".to_string()))?;
    let method = method.unwrap_or_else(|| {
        if data.is_some() || !form_fields.is_empty() {
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
        auth_user,
        auth_pass,
        form_fields,
        insecure,
    })
}

fn tokenize(input: &str) -> Result<Vec<String>, AppError> {
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

fn parse_form_field(field: &str) -> Option<(String, String)> {
    let equals_pos = field.find('=')?;
    let key = field[..equals_pos].to_string();
    let value = field[equals_pos + 1..].to_string();
    Some((key, value))
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
        let curl = r"curl https://api.example.com/users\?page=1";
        let result = parse_curl(curl).unwrap();
        assert_eq!(result.url, "https://api.example.com/users?page=1");
    }

    #[test]
    fn parse_with_basic_auth() {
        let curl = "curl -u user:pass https://api.example.com";
        let result = parse_curl(curl).unwrap();
        assert_eq!(result.auth_user.as_deref(), Some("user"));
        assert_eq!(result.auth_pass.as_deref(), Some("pass"));
    }

    #[test]
    fn parse_with_form_field() {
        let curl = r#"curl -F "file=@test.txt" -F "name=test" https://api.example.com/upload"#;
        let result = parse_curl(curl).unwrap();
        assert_eq!(result.form_fields.len(), 2);
        assert_eq!(result.form_fields[0].0, "file");
        assert_eq!(result.form_fields[0].1, "@test.txt");
        assert_eq!(result.method, "POST");
    }

    #[test]
    fn parse_with_insecure() {
        let curl = "curl -k https://self-signed.example.com";
        let result = parse_curl(curl).unwrap();
        assert!(result.insecure);
    }

    #[test]
    fn parse_with_compressed_no_duplicate() {
        let curl = r#"curl --compressed -H "Accept-Encoding: identity" https://api.example.com"#;
        let result = parse_curl(curl).unwrap();
        let accept_headers: Vec<_> = result
            .headers
            .iter()
            .filter(|(k, _)| k == "Accept-Encoding")
            .collect();
        assert_eq!(accept_headers.len(), 1);
    }

    #[test]
    fn parse_with_compressed() {
        let curl = "curl --compressed https://api.example.com";
        let result = parse_curl(curl).unwrap();
        assert!(result
            .headers
            .iter()
            .any(|(k, v)| k == "Accept-Encoding" && v == "gzip, deflate"));
    }

    #[test]
    fn parse_no_url_returns_error() {
        let curl = "curl";
        let result = parse_curl(curl);
        assert!(result.is_err());
    }

    #[test]
    fn parse_data_urlencode() {
        let curl = r#"curl --data-urlencode "comment=hello world" https://api.example.com"#;
        let result = parse_curl(curl).unwrap();
        assert_eq!(result.body.as_deref(), Some("comment=hello%20world"));
        assert_eq!(result.method, "POST");
    }

    #[test]
    fn parse_data_at_file() {
        let dir = std::env::temp_dir().join("curl_test");
        std::fs::create_dir_all(&dir).unwrap();
        let file_path = dir.join("body.json");
        std::fs::write(&file_path, r#"{"key":"value"}"#).unwrap();

        let curl = format!("curl -d @{} https://api.example.com", file_path.display());
        let result = parse_curl(&curl).unwrap();
        assert_eq!(result.body.as_deref(), Some(r#"{"key":"value"}"#));
        assert_eq!(result.method, "POST");

        std::fs::remove_file(&file_path).unwrap();
        std::fs::remove_dir(&dir).unwrap();
    }

    #[test]
    fn parse_data_binary_at_file() {
        let dir = std::env::temp_dir().join("curl_test2");
        std::fs::create_dir_all(&dir).unwrap();
        let file_path = dir.join("payload.txt");
        std::fs::write(&file_path, "binary data").unwrap();

        let curl = format!(
            "curl --data-binary @{} https://api.example.com",
            file_path.display()
        );
        let result = parse_curl(&curl).unwrap();
        assert_eq!(result.body.as_deref(), Some("binary data"));

        std::fs::remove_file(&file_path).unwrap();
        std::fs::remove_dir(&dir).unwrap();
    }

    #[test]
    fn parse_data_urlencode_at_file() {
        let dir = std::env::temp_dir().join("curl_test3");
        std::fs::create_dir_all(&dir).unwrap();
        let file_path = dir.join("query.txt");
        std::fs::write(&file_path, "search term").unwrap();

        let curl = format!(
            "curl --data-urlencode @{} https://api.example.com",
            file_path.display()
        );
        let result = parse_curl(&curl).unwrap();
        assert_eq!(result.body.as_deref(), Some("search%20term"));

        std::fs::remove_file(&file_path).unwrap();
        std::fs::remove_dir(&dir).unwrap();
    }
}
