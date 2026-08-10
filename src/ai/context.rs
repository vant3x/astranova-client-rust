use crate::ai::types::{AiChatMessage, AiRole};
use crate::http_client::request::{HttpMethod, HttpRequest};
use crate::http_client::response::HttpResponse;

/// Owned version of the context builder (no lifetimes) so it can be returned
/// from functions that build it using local HttpRequest values.
pub struct AiContextBuilder<'a> {
    pub current_request: Option<&'a HttpRequest>,
    pub current_response: Option<&'a HttpResponse>,
    pub active_env_name: Option<&'a str>,
    pub env_variables: Vec<(&'a str, &'a str)>,
    // Owned fields — used when we can't hold a reference
    owned_method: Option<String>,
    owned_url: Option<String>,
    owned_env_name: Option<String>,
    owned_env_vars: Vec<(String, String)>,
}

impl<'a> AiContextBuilder<'a> {
    pub fn new() -> Self {
        Self {
            current_request: None,
            current_response: None,
            active_env_name: None,
            env_variables: Vec::new(),
            owned_method: None,
            owned_url: None,
            owned_env_name: None,
            owned_env_vars: Vec::new(),
        }
    }

    pub fn with_request(mut self, req: &'a HttpRequest) -> Self {
        self.current_request = Some(req);
        self
    }

    pub fn with_response(mut self, resp: &'a HttpResponse) -> Self {
        self.current_response = Some(resp);
        self
    }

    pub fn with_environment(mut self, name: &'a str, vars: Vec<(&'a str, &'a str)>) -> Self {
        self.active_env_name = Some(name);
        self.env_variables = vars;
        self
    }

    /// Use owned data when the HttpRequest is a local temporary.
    pub fn with_request_owned(mut self, method: HttpMethod, url: String) -> AiContextBuilder<'static> {
        AiContextBuilder {
            current_request: None,
            current_response: None,
            active_env_name: None,
            env_variables: Vec::new(),
            owned_method: Some(method.to_string()),
            owned_url: Some(url),
            owned_env_name: self.owned_env_name,
            owned_env_vars: self.owned_env_vars,
        }
    }

    /// Use owned environment data.
    pub fn with_environment_owned(mut self, name: String, vars: Vec<(String, String)>) -> Self {
        self.owned_env_name = Some(name);
        self.owned_env_vars = vars;
        self
    }

    pub fn build_system_prompt(&self) -> String {
        let mut parts = vec![
            "You are an expert API development assistant integrated into Astraio Client, \
             a desktop HTTP/API client built in Rust."
                .to_string(),
            String::new(),
            "CAPABILITIES:".to_string(),
            "- Generate HTTP requests from natural language descriptions".to_string(),
            "- Explain API responses and debug errors".to_string(),
            "- Generate pre/post-request scripts in JavaScript (QuickJS)".to_string(),
            "- Generate mock data (JSON) for testing".to_string(),
            "- Create API test assertions".to_string(),
            "- Transform data between formats".to_string(),
            String::new(),
            "RESPONSE FORMAT:".to_string(),
            "- When generating HTTP requests, format as a clear request block with method, URL, \
             headers, and body"
                .to_string(),
            "- When generating scripts, use valid JavaScript code blocks".to_string(),
            "- When generating mock data, use valid JSON".to_string(),
            "- Be concise and practical".to_string(),
        ];

        // Check owned request info first, then borrowed reference
        let req_method = self
            .owned_method
            .as_deref()
            .or_else(|| self.current_request.map(|r| r.method.to_string()).as_deref().map(|_| ""))
            .map(|s| s.to_string());
        let req_url = self
            .owned_url
            .as_deref()
            .or_else(|| self.current_request.map(|r| r.url.as_str()))
            .map(|s| s.to_string());

        if let (Some(method), Some(url)) = (&req_method, &req_url) {
            if !method.is_empty() || !url.is_empty() {
                parts.push(String::new());
                parts.push("CURRENT REQUEST CONTEXT:".to_string());
                if !method.is_empty() {
                    parts.push(format!("  Method: {method}"));
                }
                if !url.is_empty() {
                    parts.push(format!("  URL: {url}"));
                }
            }
        } else if let Some(req) = &self.current_request {
            parts.push(String::new());
            parts.push("CURRENT REQUEST CONTEXT:".to_string());
            parts.push(format!("  Method: {}", req.method));
            parts.push(format!("  URL: {}", req.url));
            if !req.headers.is_empty() {
                parts.push("  Headers:".to_string());
                for (k, v) in &req.headers {
                    parts.push(format!("    {k}: {v}"));
                }
            }
            if let Some(body) = &req.body {
                if !body.is_empty() {
                    let truncated = if body.len() > 500 {
                        format!("{}...", &body[..500])
                    } else {
                        body.clone()
                    };
                    parts.push(format!("  Body: {truncated}"));
                }
            }
        }

        if let Some(resp) = &self.current_response {
            parts.push(String::new());
            parts.push("CURRENT RESPONSE CONTEXT:".to_string());
            parts.push(format!("  Status: {}", resp.status));
            parts.push("  Headers:".to_string());
            for (k, v) in &resp.headers {
                parts.push(format!("    {k}: {v}"));
            }
            let body = &resp.body;
            let truncated = if body.len() > 1000 {
                format!("{}...", &body[..1000])
            } else {
                body.clone()
            };
            parts.push(format!("  Body: {truncated}"));
        }

        // Environment info from owned or borrowed
        let env_name = self.owned_env_name.as_deref().or(self.active_env_name);
        let has_owned_vars = !self.owned_env_vars.is_empty();
        let has_borrowed_vars = !self.env_variables.is_empty();

        if has_owned_vars || has_borrowed_vars {
            parts.push(String::new());
            parts.push(format!(
                "ENVIRONMENT{}:",
                env_name
                    .map(|n| format!(" ({n})"))
                    .unwrap_or_default()
            ));
            for (k, v) in &self.owned_env_vars {
                parts.push(format!("  {{{{{k}}}}} = {v}"));
            }
            for (k, v) in &self.env_variables {
                parts.push(format!("  {{{{{k}}}}} = {v}"));
            }
        }

        parts.join("\n")
    }

    pub fn build_messages(
        &self,
        user_prompt: &str,
        history: &[AiChatMessage],
    ) -> Vec<AiChatMessage> {
        let mut messages = vec![AiChatMessage {
            role: AiRole::System,
            content: self.build_system_prompt(),
        }];

        for msg in history {
            messages.push(msg.clone());
        }

        messages.push(AiChatMessage {
            role: AiRole::User,
            content: user_prompt.to_string(),
        });

        messages
    }
}
