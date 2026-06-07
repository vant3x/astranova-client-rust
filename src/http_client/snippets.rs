use super::request::HttpRequest;

pub fn to_curl(request: &HttpRequest) -> String {
    let mut parts = vec!["curl".to_string()];

    if request.method != "GET" {
        parts.push(format!("-X {}", request.method));
    }

    for (key, value) in &request.headers {
        parts.push(format!(
            "-H '{}: {}'",
            key.replace('\'', "'\\''"),
            value.replace('\'', "'\\''")
        ));
    }

    if let Some(body) = &request.body {
        parts.push(format!("-d '{}'", body.replace('\'', "'\\''")));
    }

    for field in &request.multipart_fields {
        match &field.value {
            super::request::MultipartValue::Text(text) => {
                parts.push(format!(
                    "-F '{}={}'",
                    field.name.replace('\'', "'\\''"),
                    text.replace('\'', "'\\''")
                ));
            }
            super::request::MultipartValue::File { path, .. } => {
                parts.push(format!(
                    "-F '{}=@{}'",
                    field.name.replace('\'', "'\\''"),
                    path
                ));
            }
        }
    }

    parts.push(format!("'{}'", request.url));

    parts.join(" \\\n  ")
}

pub fn to_python(request: &HttpRequest) -> String {
    let mut lines = vec!["import requests".to_string(), String::new()];

    let method = request.method.to_lowercase();

    let mut kwargs = vec![];

    if !request.headers.is_empty() {
        let headers: Vec<String> = request
            .headers
            .iter()
            .map(|(k, v)| format!("    \"{}\": \"{}\"", k, v))
            .collect();
        lines.push("headers = {".to_string());
        lines.extend(headers);
        lines.push("}".to_string());
        lines.push(String::new());
        kwargs.push("headers=headers");
    }

    if let Some(body) = &request.body {
        if is_json(body) {
            lines.push(format!("json_data = {}", body));
            lines.push(String::new());
            kwargs.push("json=json_data");
        } else {
            lines.push(format!(
                "data = \"{}\"",
                body.replace('\\', "\\\\").replace('"', "\\\"")
            ));
            lines.push(String::new());
            kwargs.push("data=data");
        }
    }

    if !request.multipart_fields.is_empty() {
        let files: Vec<String> = request
            .multipart_fields
            .iter()
            .filter(|f| !f.name.is_empty())
            .map(|f| match &f.value {
                super::request::MultipartValue::Text(text) => {
                    format!("    \"{}\": (None, \"{}\")", f.name, text)
                }
                super::request::MultipartValue::File { path, .. } => {
                    format!("    \"{}\": open(\"{}\", \"rb\")", f.name, path)
                }
            })
            .collect();
        lines.push("files = {".to_string());
        lines.extend(files);
        lines.push("}".to_string());
        lines.push(String::new());
        kwargs.push("files=files");
    }

    let kwargs_str = if kwargs.is_empty() {
        String::new()
    } else {
        format!(", {}", kwargs.join(", "))
    };

    lines.push(format!(
        "response = requests.{}(\"{}\"{})",
        method, request.url, kwargs_str
    ));
    lines.push(String::new());
    lines.push("print(response.status_code)".to_string());
    lines.push("print(response.text)".to_string());

    lines.join("\n")
}

pub fn to_javascript(request: &HttpRequest) -> String {
    let mut lines = vec![];

    let method = request.method.to_uppercase();

    let mut options = vec![format!("    method: \"{}\"", method)];

    if !request.headers.is_empty() {
        let headers: Vec<String> = request
            .headers
            .iter()
            .map(|(k, v)| format!("    \"{}\": \"{}\"", k, v))
            .collect();
        options.push("    headers: {".to_string());
        for h in &headers {
            options.push(format!("{}{},", " ".repeat(8), h));
        }
        options.push("    }".to_string());
    }

    if let Some(body) = &request.body {
        if is_json(body) {
            options.push(format!("    body: JSON.stringify({})", body));
        } else {
            options.push(format!("    body: \"{}\"", body.replace('"', "\\\"")));
        }
    }

    if !request.multipart_fields.is_empty() {
        lines.push("const formData = new FormData();".to_string());
        for field in &request.multipart_fields {
            match &field.value {
                super::request::MultipartValue::Text(text) => {
                    lines.push(format!(
                        "formData.append(\"{}\", \"{}\");",
                        field.name, text
                    ));
                }
                super::request::MultipartValue::File { path, .. } => {
                    lines.push(format!(
                        "formData.append(\"{}\", fileInput.files[0]); // {}",
                        field.name, path
                    ));
                }
            }
        }
        lines.push(String::new());
        options.push("    body: formData".to_string());
    }

    let options_str = options.join(",\n");

    lines.push(format!("fetch(\"{}\", {{", request.url));
    lines.push(options_str);
    lines.push("})".to_string());
    lines.push("  .then(response => response.text())".to_string());
    lines.push("  .then(data => console.log(data))".to_string());
    lines.push("  .catch(error => console.error(error));".to_string());

    lines.join("\n")
}

pub fn to_rust(request: &HttpRequest) -> String {
    let mut lines = vec![];
    let method = request.method.to_lowercase();

    lines.push("use reqwest;".to_string());
    lines.push(String::new());
    lines.push("#[tokio::main]".to_string());
    lines.push("async fn main() -> Result<(), Box<dyn std::error::Error>> {".to_string());

    let mut args = vec![format!("\"{}\"", request.url)];

    if request.method != "GET" {
        args.push(format!(
            ".method(reqwest::Method::{})",
            method.to_uppercase()
        ));
    }

    if !request.headers.is_empty() {
        lines.push("    let client = reqwest::Client::new();".to_string());
        lines.push(format!(
            "    let resp = client.{}(\"{}\")",
            method, request.url
        ));
        for (key, value) in &request.headers {
            lines.push(format!("        .header(\"{}\", \"{}\")", key, value));
        }
        if let Some(body) = &request.body {
            if is_json(body) {
                lines.push(format!("        .json(&{})", body));
            } else {
                lines.push(format!("        .body(\"{}\")", body.replace('"', "\\\"")));
            }
        }
        lines.push("        .send()".to_string());
        lines.push("        .await?;".to_string());
    } else {
        lines.push(format!(
            "    let resp = reqwest::get(\"{}\").await?;",
            request.url
        ));
    }

    lines.push(String::new());
    lines.push("    println!(\"Status: {}\", resp.status());".to_string());
    lines.push("    println!(\"Body: {}\", resp.text().await?);".to_string());
    lines.push("    Ok(())".to_string());
    lines.push("}".to_string());

    lines.join("\n")
}

fn is_json(s: &str) -> bool {
    let trimmed = s.trim();
    (trimmed.starts_with('{') && trimmed.ends_with('}'))
        || (trimmed.starts_with('[') && trimmed.ends_with(']'))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnippetFormat {
    Curl,
    Python,
    JavaScript,
    Rust,
}

impl SnippetFormat {
    pub const ALL: [SnippetFormat; 4] = [
        SnippetFormat::Curl,
        SnippetFormat::Python,
        SnippetFormat::JavaScript,
        SnippetFormat::Rust,
    ];
}

impl std::fmt::Display for SnippetFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SnippetFormat::Curl => write!(f, "cURL"),
            SnippetFormat::Python => write!(f, "Python"),
            SnippetFormat::JavaScript => write!(f, "JavaScript"),
            SnippetFormat::Rust => write!(f, "Rust"),
        }
    }
}

pub fn generate(request: &HttpRequest, format: SnippetFormat) -> String {
    match format {
        SnippetFormat::Curl => to_curl(request),
        SnippetFormat::Python => to_python(request),
        SnippetFormat::JavaScript => to_javascript(request),
        SnippetFormat::Rust => to_rust(request),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::http_client::config::RequestConfig;

    fn make_request(method: &str, url: &str) -> HttpRequest {
        HttpRequest {
            method: method.to_string(),
            url: url.to_string(),
            headers: vec![],
            body: None,
            config: RequestConfig::default(),
            multipart_fields: vec![],
        }
    }

    #[test]
    fn curl_get_simple() {
        let req = make_request("GET", "https://api.example.com/users");
        let curl = to_curl(&req);
        assert!(curl.contains("curl"));
        assert!(curl.contains("'https://api.example.com/users'"));
        assert!(!curl.contains("-X"));
    }

    #[test]
    fn curl_post_with_body() {
        let mut req = make_request("POST", "https://api.example.com/users");
        req.body = Some(r#"{"name": "John"}"#.to_string());
        req.headers
            .push(("Content-Type".to_string(), "application/json".to_string()));
        let curl = to_curl(&req);
        assert!(curl.contains("-X POST"));
        assert!(curl.contains("-H 'Content-Type: application/json'"));
        assert!(curl.contains("-d '{\"name\": \"John\"}'"));
    }

    #[test]
    fn curl_with_special_chars() {
        let mut req = make_request("POST", "https://api.example.com");
        req.body = Some("it's a test".to_string());
        let curl = to_curl(&req);
        assert!(curl.contains("'it'\\''s a test'"));
    }

    #[test]
    fn python_get() {
        let req = make_request("GET", "https://api.example.com/data");
        let py = to_python(&req);
        assert!(py.contains("import requests"));
        assert!(py.contains("requests.get(\"https://api.example.com/data\")"));
    }

    #[test]
    fn python_post_json() {
        let mut req = make_request("POST", "https://api.example.com/users");
        req.body = Some(r#"{"name": "John"}"#.to_string());
        let py = to_python(&req);
        assert!(py.contains("json_data = {\"name\": \"John\"}"));
        assert!(py.contains("json=json_data"));
    }

    #[test]
    fn python_with_headers() {
        let mut req = make_request("GET", "https://api.example.com");
        req.headers
            .push(("Authorization".to_string(), "Bearer token123".to_string()));
        let py = to_python(&req);
        assert!(py.contains("\"Authorization\": \"Bearer token123\""));
        assert!(py.contains("headers=headers"));
    }

    #[test]
    fn javascript_get() {
        let req = make_request("GET", "https://api.example.com/data");
        let js = to_javascript(&req);
        assert!(js.contains("fetch(\"https://api.example.com/data\""));
        assert!(js.contains("method: \"GET\""));
    }

    #[test]
    fn javascript_post_json() {
        let mut req = make_request("POST", "https://api.example.com/users");
        req.body = Some(r#"{"name": "John"}"#.to_string());
        let js = to_javascript(&req);
        assert!(js.contains("JSON.stringify({\"name\": \"John\"})"));
    }

    #[test]
    fn rust_get() {
        let req = make_request("GET", "https://api.example.com/data");
        let rust = to_rust(&req);
        assert!(rust.contains("reqwest::get(\"https://api.example.com/data\")"));
        assert!(rust.contains("resp.status()"));
    }

    #[test]
    fn snippet_format_display() {
        assert_eq!(SnippetFormat::Curl.to_string(), "cURL");
        assert_eq!(SnippetFormat::Python.to_string(), "Python");
        assert_eq!(SnippetFormat::JavaScript.to_string(), "JavaScript");
        assert_eq!(SnippetFormat::Rust.to_string(), "Rust");
    }

    #[test]
    fn generate_returns_correct_format() {
        let req = make_request("GET", "https://example.com");
        assert_eq!(generate(&req, SnippetFormat::Curl), to_curl(&req));
        assert_eq!(generate(&req, SnippetFormat::Python), to_python(&req));
        assert_eq!(
            generate(&req, SnippetFormat::JavaScript),
            to_javascript(&req)
        );
        assert_eq!(generate(&req, SnippetFormat::Rust), to_rust(&req));
    }

    #[test]
    fn is_json_detects_objects() {
        assert!(is_json(r#"{"key": "value"}"#));
        assert!(is_json(r#"[1, 2, 3]"#));
        assert!(is_json("  { }  "));
        assert!(!is_json("hello"));
        assert!(!is_json("<html></html>"));
    }

    #[test]
    fn curl_with_multipart() {
        let mut req = make_request("POST", "https://api.example.com/upload");
        req.multipart_fields
            .push(crate::http_client::request::MultipartField {
                name: "file".to_string(),
                value: crate::http_client::request::MultipartValue::File {
                    path: "/tmp/test.pdf".to_string(),
                    filename: Some("test.pdf".to_string()),
                },
            });
        req.multipart_fields
            .push(crate::http_client::request::MultipartField {
                name: "description".to_string(),
                value: crate::http_client::request::MultipartValue::Text("My file".to_string()),
            });
        let curl = to_curl(&req);
        assert!(curl.contains("-F 'file=@/tmp/test.pdf'"));
        assert!(curl.contains("-F 'description=My file'"));
    }
}
