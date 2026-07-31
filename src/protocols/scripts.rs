use crate::error::AppError;
use crate::http_client::request::HttpRequest;
use crate::http_client::response::HttpResponse;
use base64::Engine;
use chrono::Utc;
use hmac::{Hmac, Mac};
use rand::Rng;
use regex::Regex;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;

type HmacSha256 = Hmac<Sha256>;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum ScriptAction {
    SetVariable {
        name: String,
        value: String,
    },
    SetHeader {
        key: String,
        value: String,
    },
    RemoveHeader {
        key: String,
    },
    SetBody {
        body: String,
    },
    SetUrl {
        url: String,
    },
    SetMethod {
        method: String,
    },
    AssertStatus {
        expected: u16,
    },
    AssertHeader {
        key: String,
        contains: Option<String>,
        equals: Option<String>,
    },
    AssertBody {
        contains: Option<String>,
        equals: Option<String>,
    },
    ExtractJson {
        variable: String,
        path: String,
    },
    ExtractHeader {
        variable: String,
        header: String,
    },
    Log {
        message: String,
    },
    Delay {
        ms: u64,
    },
    TransformToUpper {
        input: String,
        variable: String,
    },
    TransformToLower {
        input: String,
        variable: String,
    },
    TransformTrim {
        input: String,
        variable: String,
    },
    EncodeBase64 {
        input: String,
        variable: String,
    },
    DecodeBase64 {
        input: String,
        variable: String,
    },
    HashSha256 {
        input: String,
        variable: String,
    },
    HmacSha256 {
        key: String,
        message: String,
        variable: String,
    },
    IfStatus {
        code: u16,
        then: Vec<ScriptAction>,
        #[serde(default, rename = "else")]
        else_actions: Option<Vec<ScriptAction>>,
    },
    ExtractRegex {
        variable: String,
        pattern: String,
    },
    SetBodyJson {
        path: String,
        value: String,
    },
    AssertJsonPath {
        path: String,
        equals: Option<String>,
        contains: Option<String>,
    },
    SetQuery {
        key: String,
        value: String,
    },
}

impl std::fmt::Display for ScriptAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScriptAction::SetVariable { name, value } => write!(f, "set_var({}={})", name, value),
            ScriptAction::SetHeader { key, value } => write!(f, "set_header({}: {})", key, value),
            ScriptAction::RemoveHeader { key } => write!(f, "remove_header({})", key),
            ScriptAction::SetBody { body } => {
                let preview: String = body.chars().take(30).collect();
                write!(f, "set_body({}...)", preview)
            }
            ScriptAction::SetUrl { url } => write!(f, "set_url({})", url),
            ScriptAction::SetMethod { method } => write!(f, "set_method({})", method),
            ScriptAction::AssertStatus { expected } => write!(f, "assert_status({})", expected),
            ScriptAction::AssertHeader { key, .. } => write!(f, "assert_header({})", key),
            ScriptAction::AssertBody { .. } => write!(f, "assert_body(...)"),
            ScriptAction::ExtractJson { variable, path } => {
                write!(f, "extract_json({} from {})", variable, path)
            }
            ScriptAction::ExtractHeader { variable, header } => {
                write!(f, "extract_header({} from {})", variable, header)
            }
            ScriptAction::Log { message } => write!(f, "log({})", message),
            ScriptAction::Delay { ms } => write!(f, "delay({}ms)", ms),
            ScriptAction::TransformToUpper { input, .. } => write!(f, "to_upper({})", input),
            ScriptAction::TransformToLower { input, .. } => write!(f, "to_lower({})", input),
            ScriptAction::TransformTrim { input, .. } => write!(f, "trim({})", input),
            ScriptAction::EncodeBase64 { input, .. } => write!(f, "base64_encode({})", input),
            ScriptAction::DecodeBase64 { input, .. } => write!(f, "base64_decode({})", input),
            ScriptAction::HashSha256 { input, .. } => write!(f, "sha256({})", input),
            ScriptAction::HmacSha256 { message, .. } => write!(f, "hmac_sha256({})", message),
            ScriptAction::IfStatus { code, .. } => write!(f, "if_status({})", code),
            ScriptAction::ExtractRegex { variable, pattern } => {
                write!(f, "extract_regex({} from {})", variable, pattern)
            }
            ScriptAction::SetBodyJson { path, value } => {
                write!(f, "set_body_json({}={})", path, value)
            }
            ScriptAction::AssertJsonPath { path, .. } => write!(f, "assert_json_path({})", path),
            ScriptAction::SetQuery { key, value } => write!(f, "set_query({}={})", key, value),
        }
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq)]
#[serde(transparent)]
pub struct Script {
    pub actions: Vec<ScriptAction>,
}

impl Script {
    pub fn from_json(json: &str) -> Result<Self, AppError> {
        if json.trim().is_empty() {
            return Ok(Self::default());
        }
        serde_json::from_str(json).map_err(|e| AppError::Parse(format!("Invalid script: {}", e)))
    }

    pub fn to_json(&self) -> Result<String, AppError> {
        serde_json::to_string_pretty(self).map_err(|e| AppError::Serialization(e.to_string()))
    }
}

#[derive(Debug, Clone)]
pub struct ScriptContext {
    pub variables: HashMap<String, String>,
    pub logs: Vec<String>,
    pub errors: Vec<String>,
}

impl ScriptContext {
    pub fn new() -> Self {
        Self {
            variables: HashMap::new(),
            logs: Vec::new(),
            errors: Vec::new(),
        }
    }

    pub fn resolve_variables(&mut self, input: &str) -> String {
        let mut result = input.to_string();
        let max_iterations = 10;

        for _ in 0..max_iterations {
            let mut changed = false;

            for (key, value) in &self.variables {
                let placeholder = format!("{{{{{}}}}}", key);
                if result.contains(&placeholder) {
                    result = result.replace(&placeholder, value);
                    changed = true;
                }
            }

            let now = Utc::now();
            let dynamic_tokens: Vec<(&str, String)> = vec![
                ("{{$timestamp}}", now.timestamp().to_string()),
                ("{{$isoNow}}", now.to_rfc3339()),
                (
                    "{{$randomInt}}",
                    rand::thread_rng().gen::<u32>().to_string(),
                ),
                ("{{$uuid}}", generate_uuid_v4()),
            ];

            for (token, value) in &dynamic_tokens {
                if result.contains(token) {
                    result = result.replace(token, value);
                    changed = true;
                }
            }

            if !changed {
                break;
            }
        }

        let mut remaining = result.as_str();
        while let Some(start) = remaining.find("{{") {
            if let Some(end) = remaining[start..].find("}}") {
                let var_name = &remaining[start + 2..start + end];
                self.errors
                    .push(format!("Unresolved variable: {{{{ {} }}}}", var_name));
                remaining = &remaining[start + end + 2..];
            } else {
                break;
            }
        }

        result
    }
}

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct RequestScripts {
    #[serde(default)]
    pub pre_request: Script,
    #[serde(default)]
    pub post_response: Script,
    #[serde(default)]
    pub js_pre_request: String,
    #[serde(default)]
    pub js_post_response: String,
}

impl RequestScripts {
    pub fn from_json(json: &str) -> Result<Self, AppError> {
        if json.trim().is_empty() {
            return Ok(Self::default());
        }
        serde_json::from_str(json).map_err(|e| AppError::Parse(format!("Invalid scripts: {}", e)))
    }

    pub fn to_json(&self) -> Result<String, AppError> {
        serde_json::to_string_pretty(self).map_err(|e| AppError::Serialization(e.to_string()))
    }
}

pub struct ScriptEngine;

impl ScriptEngine {
    pub fn execute_pre_request(
        script: &Script,
        request: &mut HttpRequest,
        context: &mut ScriptContext,
    ) -> Result<(), AppError> {
        for action in &script.actions {
            Self::execute_action_pre(action, request, context)?;
        }
        Ok(())
    }

    pub fn execute_post_response(
        script: &Script,
        response: &HttpResponse,
        context: &mut ScriptContext,
    ) -> Result<(), AppError> {
        let mut errors = Vec::new();
        for action in &script.actions {
            if let Err(e) = Self::execute_action_post(action, response, context) {
                errors.push(e.to_string());
            }
        }
        if !errors.is_empty() {
            Err(AppError::Validation(errors.join("\n")))
        } else {
            Ok(())
        }
    }

    /// Execute actions shared by both pre-request and post-response scripts.
    /// Returns `Ok(true)` if the action was handled, `Ok(false)` if the caller should handle it.
    fn execute_shared_action(
        action: &ScriptAction,
        context: &mut ScriptContext,
    ) -> Result<bool, AppError> {
        match action {
            ScriptAction::SetVariable { name, value } => {
                let resolved = context.resolve_variables(value);
                context.variables.insert(name.clone(), resolved);
            }
            ScriptAction::Log { message } => {
                let resolved = context.resolve_variables(message);
                context.logs.push(resolved);
            }
            ScriptAction::TransformToUpper { input, variable } => {
                let resolved_input = context.resolve_variables(input);
                context
                    .variables
                    .insert(variable.clone(), resolved_input.to_uppercase());
            }
            ScriptAction::TransformToLower { input, variable } => {
                let resolved_input = context.resolve_variables(input);
                context
                    .variables
                    .insert(variable.clone(), resolved_input.to_lowercase());
            }
            ScriptAction::TransformTrim { input, variable } => {
                let resolved_input = context.resolve_variables(input);
                context
                    .variables
                    .insert(variable.clone(), resolved_input.trim().to_string());
            }
            ScriptAction::EncodeBase64 { input, variable } => {
                let resolved_input = context.resolve_variables(input);
                let encoded =
                    base64::engine::general_purpose::STANDARD.encode(resolved_input.as_bytes());
                context.variables.insert(variable.clone(), encoded);
            }
            ScriptAction::DecodeBase64 { input, variable } => {
                let resolved_input = context.resolve_variables(input);
                match base64::engine::general_purpose::STANDARD.decode(&resolved_input) {
                    Ok(bytes) => {
                        let decoded = String::from_utf8_lossy(&bytes).to_string();
                        context.variables.insert(variable.clone(), decoded);
                    }
                    Err(e) => {
                        let msg = format!("Base64 decode failed: {}", e);
                        context.errors.push(msg.clone());
                        return Err(AppError::Validation(msg));
                    }
                }
            }
            ScriptAction::HashSha256 { input, variable } => {
                let resolved_input = context.resolve_variables(input);
                let mut hasher = Sha256::new();
                hasher.update(resolved_input.as_bytes());
                let result = format!("{:x}", hasher.finalize());
                context.variables.insert(variable.clone(), result);
            }
            ScriptAction::HmacSha256 {
                key,
                message,
                variable,
            } => {
                let resolved_key = context.resolve_variables(key);
                let resolved_message = context.resolve_variables(message);
                let mut mac = HmacSha256::new_from_slice(resolved_key.as_bytes())
                    .map_err(|e| AppError::Validation(format!("HMAC key error: {}", e)))?;
                mac.update(resolved_message.as_bytes());
                let result = format!("{:x}", mac.finalize().into_bytes());
                context.variables.insert(variable.clone(), result);
            }
            _ => return Ok(false),
        }
        Ok(true)
    }

    fn execute_action_pre(
        action: &ScriptAction,
        request: &mut HttpRequest,
        context: &mut ScriptContext,
    ) -> Result<(), AppError> {
        if Self::execute_shared_action(action, context)? {
            return Ok(());
        }

        match action {
            ScriptAction::SetHeader { key, value } => {
                let resolved_key = context.resolve_variables(key);
                let resolved_value = context.resolve_variables(value);
                request
                    .headers
                    .retain(|(k, _)| !k.eq_ignore_ascii_case(&resolved_key));
                request.headers.push((resolved_key, resolved_value));
            }
            ScriptAction::RemoveHeader { key } => {
                let resolved = context.resolve_variables(key);
                request
                    .headers
                    .retain(|(k, _)| !k.eq_ignore_ascii_case(&resolved));
            }
            ScriptAction::SetBody { body } => {
                let resolved = context.resolve_variables(body);
                request.body = Some(resolved);
            }
            ScriptAction::SetUrl { url } => {
                let resolved = context.resolve_variables(url);
                request.url = resolved;
            }
            ScriptAction::SetMethod { method } => {
                let resolved = context.resolve_variables(method);
                let upper = resolved.to_uppercase();
                let valid = matches!(
                    upper.as_str(),
                    "GET"
                        | "POST"
                        | "PUT"
                        | "DELETE"
                        | "PATCH"
                        | "HEAD"
                        | "OPTIONS"
                        | "TRACE"
                        | "CONNECT"
                );
                if !valid {
                    return Err(AppError::Validation(format!(
                        "Invalid HTTP method: {}",
                        resolved
                    )));
                }
                request.method = resolved.parse().map_err(|_| {
                    AppError::Validation(format!("Invalid HTTP method: {}", resolved))
                })?;
            }
            ScriptAction::Delay { ms } => {
                log::info!("Pre-request delay: {}ms (applied by request handler)", ms);
            }
            ScriptAction::IfStatus { .. } => {
                log::warn!(
                    "IfStatus is only valid in post-response scripts, skipping in pre-request"
                );
            }
            ScriptAction::ExtractRegex { variable, pattern } => {
                let resolved_pattern = context.resolve_variables(pattern);
                match Regex::new(&resolved_pattern) {
                    Ok(re) => {
                        if let Some(caps) = re.captures(&request.url) {
                            let val = caps
                                .get(1)
                                .map(|m| m.as_str().to_string())
                                .or_else(|| caps.get(0).map(|m| m.as_str().to_string()))
                                .unwrap_or_default();
                            context.variables.insert(variable.clone(), val);
                        }
                    }
                    Err(e) => {
                        return Err(AppError::Validation(format!("Invalid regex: {}", e)));
                    }
                }
            }
            ScriptAction::SetBodyJson { path, value } => {
                let resolved_path = context.resolve_variables(path);
                let resolved_value = context.resolve_variables(value);
                let current_body = request.body.as_deref().unwrap_or("{}");
                let mut json: serde_json::Value = serde_json::from_str(current_body)
                    .unwrap_or(serde_json::Value::Object(serde_json::Map::new()));
                let new_val: serde_json::Value = serde_json::from_str(&resolved_value)
                    .unwrap_or(serde_json::Value::String(resolved_value.clone()));
                set_json_path(&mut json, &resolved_path, new_val);
                request.body = Some(serde_json::to_string(&json).unwrap_or_default());
            }
            ScriptAction::SetQuery { key, value } => {
                let resolved_key = context.resolve_variables(key);
                let resolved_value = context.resolve_variables(value);
                if let Ok(mut url) = reqwest::Url::parse(&request.url) {
                    url.query_pairs_mut()
                        .append_pair(&resolved_key, &resolved_value);
                    request.url = url.to_string();
                } else {
                    let separator = if request.url.contains('?') { '&' } else { '?' };
                    request.url = format!(
                        "{}{}{}={}",
                        request.url, separator, resolved_key, resolved_value
                    );
                }
            }
            ScriptAction::AssertJsonPath { .. } => {
                log::warn!("AssertJsonPath is only valid in post-response scripts, skipping in pre-request");
            }
            _ => {}
        }
        Ok(())
    }

    fn execute_action_post(
        action: &ScriptAction,
        response: &HttpResponse,
        context: &mut ScriptContext,
    ) -> Result<(), AppError> {
        if Self::execute_shared_action(action, context)? {
            return Ok(());
        }

        match action {
            ScriptAction::AssertStatus { expected } => {
                if response.status != *expected {
                    let msg = format!(
                        "Assertion failed: expected status {}, got {}",
                        expected, response.status
                    );
                    context.errors.push(msg.clone());
                    return Err(AppError::Validation(msg));
                }
            }
            ScriptAction::AssertHeader {
                key,
                contains,
                equals,
            } => {
                let header_value = response
                    .headers
                    .iter()
                    .find(|(k, _)| k.eq_ignore_ascii_case(key))
                    .map(|(_, v)| v.as_str());

                match header_value {
                    None => {
                        let msg = format!("Assertion failed: header '{}' not found", key);
                        context.errors.push(msg.clone());
                        return Err(AppError::Validation(msg));
                    }
                    Some(val) => {
                        if let Some(expected) = equals {
                            let resolved = context.resolve_variables(expected);
                            if val != resolved {
                                let msg = format!(
                                    "Assertion failed: header '{}' expected '{}', got '{}'",
                                    key, resolved, val
                                );
                                context.errors.push(msg.clone());
                                return Err(AppError::Validation(msg));
                            }
                        }
                        if let Some(expected) = contains {
                            let resolved = context.resolve_variables(expected);
                            if !val.contains(resolved.as_str()) {
                                let msg = format!(
                                    "Assertion failed: header '{}' should contain '{}', got '{}'",
                                    key, resolved, val
                                );
                                context.errors.push(msg.clone());
                                return Err(AppError::Validation(msg));
                            }
                        }
                    }
                }
            }
            ScriptAction::AssertBody { contains, equals } => {
                if let Some(expected) = equals {
                    let resolved = context.resolve_variables(expected);
                    if response.body != resolved {
                        let msg = format!(
                            "Assertion failed: body expected '{}', got '{}'",
                            resolved,
                            &response.body[..response.body.len().min(100)]
                        );
                        context.errors.push(msg.clone());
                        return Err(AppError::Validation(msg));
                    }
                }
                if let Some(expected) = contains {
                    let resolved = context.resolve_variables(expected);
                    if !response.body.contains(resolved.as_str()) {
                        let msg = format!("Assertion failed: body should contain '{}'", resolved);
                        context.errors.push(msg.clone());
                        return Err(AppError::Validation(msg));
                    }
                }
            }
            ScriptAction::ExtractJson { variable, path } => {
                let resolved_path = context.resolve_variables(path);
                if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(&response.body) {
                    let extracted = extract_json_path(&json_value, &resolved_path);
                    if let Some(val) = extracted {
                        context.variables.insert(variable.clone(), val);
                    } else {
                        context
                            .errors
                            .push(format!("JSON path '{}' not found", resolved_path));
                    }
                }
            }
            ScriptAction::ExtractHeader { variable, header } => {
                let resolved_header = context.resolve_variables(header);
                if let Some((_, value)) = response
                    .headers
                    .iter()
                    .find(|(k, _)| k.eq_ignore_ascii_case(&resolved_header))
                {
                    context.variables.insert(variable.clone(), value.clone());
                } else {
                    context
                        .errors
                        .push(format!("Header '{}' not found", resolved_header));
                }
            }
            ScriptAction::IfStatus {
                code,
                then,
                else_actions,
            } => {
                if response.status == *code {
                    for action in then {
                        if let Err(e) = Self::execute_action_post(action, response, context) {
                            context.errors.push(e.to_string());
                        }
                    }
                } else if let Some(else_acts) = else_actions {
                    for action in else_acts {
                        if let Err(e) = Self::execute_action_post(action, response, context) {
                            context.errors.push(e.to_string());
                        }
                    }
                }
            }
            ScriptAction::Delay { .. } => {
                log::warn!(
                    "Delay is only meaningful in pre-request scripts, skipping in post-response"
                );
            }
            ScriptAction::ExtractRegex { variable, pattern } => {
                let resolved_pattern = context.resolve_variables(pattern);
                match Regex::new(&resolved_pattern) {
                    Ok(re) => {
                        if let Some(caps) = re.captures(&response.body) {
                            let val = caps
                                .get(1)
                                .map(|m| m.as_str().to_string())
                                .or_else(|| caps.get(0).map(|m| m.as_str().to_string()))
                                .unwrap_or_default();
                            context.variables.insert(variable.clone(), val);
                        } else {
                            context.errors.push(format!(
                                "Regex pattern '{}' not found in response",
                                resolved_pattern
                            ));
                        }
                    }
                    Err(e) => {
                        let msg = format!("Invalid regex '{}': {}", resolved_pattern, e);
                        context.errors.push(msg.clone());
                        return Err(AppError::Validation(msg));
                    }
                }
            }
            ScriptAction::AssertJsonPath {
                path,
                equals,
                contains,
            } => {
                let resolved_path = context.resolve_variables(path);
                if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(&response.body) {
                    if let Some(extracted) = extract_json_path(&json_value, &resolved_path) {
                        if let Some(expected) = equals {
                            let resolved_expected = context.resolve_variables(expected);
                            if extracted != resolved_expected {
                                let msg = format!(
                                    "AssertJsonPath '{}' expected '{}', got '{}'",
                                    resolved_path, resolved_expected, extracted
                                );
                                context.errors.push(msg.clone());
                                return Err(AppError::Validation(msg));
                            }
                        }
                        if let Some(expected) = contains {
                            let resolved_expected = context.resolve_variables(expected);
                            if !extracted.contains(resolved_expected.as_str()) {
                                let msg = format!(
                                    "AssertJsonPath '{}' should contain '{}', got '{}'",
                                    resolved_path, resolved_expected, extracted
                                );
                                context.errors.push(msg.clone());
                                return Err(AppError::Validation(msg));
                            }
                        }
                    } else {
                        let msg = format!("AssertJsonPath: path '{}' not found", resolved_path);
                        context.errors.push(msg.clone());
                        return Err(AppError::Validation(msg));
                    }
                }
            }
            ScriptAction::SetBodyJson { .. } | ScriptAction::SetQuery { .. } => {
                log::warn!("SetBodyJson/SetQuery are only meaningful in pre-request scripts, skipping in post-response");
            }
            _ => {}
        }
        Ok(())
    }
}

enum PathSegment {
    Key(String),
    Index(usize),
}

fn parse_json_path_segments(path: &str) -> Vec<PathSegment> {
    let mut segments = Vec::new();
    for part in path.split('.') {
        if let Some(bracket_pos) = part.find('[') {
            let key = &part[..bracket_pos];
            if !key.is_empty() {
                segments.push(PathSegment::Key(key.to_string()));
            }
            let rest = &part[bracket_pos..];
            for bracket_content in rest.split(']').filter(|s| s.starts_with('[')) {
                let idx_str = &bracket_content[1..];
                if let Ok(idx) = idx_str.parse::<usize>() {
                    segments.push(PathSegment::Index(idx));
                }
            }
        } else if let Ok(idx) = part.parse::<usize>() {
            segments.push(PathSegment::Index(idx));
        } else {
            segments.push(PathSegment::Key(part.to_string()));
        }
    }
    segments
}

fn extract_json_path(value: &serde_json::Value, path: &str) -> Option<String> {
    let segments = parse_json_path_segments(path);
    let mut current = value;

    for segment in segments {
        match segment {
            PathSegment::Key(key) => {
                current = current.get(key.as_str())?;
            }
            PathSegment::Index(idx) => {
                current = current.get(idx)?;
            }
        }
    }

    match current {
        serde_json::Value::String(s) => Some(s.clone()),
        other => Some(other.to_string()),
    }
}

fn set_json_path(value: &mut serde_json::Value, path: &str, new_val: serde_json::Value) {
    let segments = parse_json_path_segments(path);
    let mut current = value;

    for (i, segment) in segments.iter().enumerate() {
        let is_last = i == segments.len() - 1;
        match segment {
            PathSegment::Key(key) => {
                if is_last {
                    if let serde_json::Value::Object(map) = current {
                        map.insert(key.clone(), new_val);
                        return;
                    }
                } else {
                    if let serde_json::Value::Object(map) = current {
                        current = map
                            .entry(key.clone())
                            .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()));
                    } else {
                        return;
                    }
                }
            }
            PathSegment::Index(idx) => {
                if is_last {
                    if let serde_json::Value::Array(arr) = current {
                        if *idx <= arr.len() {
                            arr.insert(*idx, new_val);
                            return;
                        }
                    }
                } else {
                    if let serde_json::Value::Array(arr) = current {
                        if *idx < arr.len() {
                            current = &mut arr[*idx];
                        } else {
                            return;
                        }
                    } else {
                        return;
                    }
                }
            }
        }
    }
}

fn generate_uuid_v4() -> String {
    let mut rng = rand::thread_rng();
    let mut bytes = [0u8; 16];
    rng.fill(&mut bytes);
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        bytes[0], bytes[1], bytes[2], bytes[3],
        bytes[4], bytes[5],
        bytes[6], bytes[7],
        bytes[8], bytes[9],
        bytes[10], bytes[11], bytes[12], bytes[13], bytes[14], bytes[15]
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::http_client::config::RequestConfig;
    use crate::http_client::request::HttpMethod;
    use crate::http_client::response::BodyEncoding;
    use std::time::Duration;

    fn make_request() -> HttpRequest {
        HttpRequest {
            method: HttpMethod::Get,
            url: "https://api.example.com/users".to_string(),
            headers: vec![],
            body: None,
            config: RequestConfig::default(),
            multipart_fields: vec![],
            auth: None,
        }
    }

    fn make_response(status: u16, body: &str) -> HttpResponse {
        HttpResponse {
            url: "https://api.example.com/users".to_string(),
            method: HttpMethod::Get,
            status,
            headers: vec![("content-type".to_string(), "application/json".to_string())],
            body: body.to_string(),
            body_encoding: BodyEncoding::Text,
            duration: Duration::from_millis(100),
            size: body.len() as u64,
            redirect_chain: vec![],
        }
    }

    #[test]
    fn script_from_json_empty() {
        let script = Script::from_json("").unwrap();
        assert!(script.actions.is_empty());
    }

    #[test]
    fn script_from_json_valid() {
        let json = r#"[
            {"action": "set_variable", "name": "token", "value": "abc123"},
            {"action": "set_header", "key": "Authorization", "value": "Bearer {{token}}"}
        ]"#;
        let script = Script::from_json(json).unwrap();
        assert_eq!(script.actions.len(), 2);
    }

    #[test]
    fn script_from_json_invalid() {
        let result = Script::from_json("not json");
        assert!(result.is_err());
    }

    #[test]
    fn set_variable_pre_request() {
        let script = Script {
            actions: vec![ScriptAction::SetVariable {
                name: "myvar".to_string(),
                value: "hello".to_string(),
            }],
        };
        let mut request = make_request();
        let mut context = ScriptContext::new();

        ScriptEngine::execute_pre_request(&script, &mut request, &mut context).unwrap();
        assert_eq!(context.variables.get("myvar").unwrap(), "hello");
    }

    #[test]
    fn set_header_pre_request() {
        let script = Script {
            actions: vec![ScriptAction::SetHeader {
                key: "X-Custom".to_string(),
                value: "test-value".to_string(),
            }],
        };
        let mut request = make_request();
        let mut context = ScriptContext::new();

        ScriptEngine::execute_pre_request(&script, &mut request, &mut context).unwrap();
        assert!(request
            .headers
            .iter()
            .any(|(k, v)| k == "X-Custom" && v == "test-value"));
    }

    #[test]
    fn set_header_with_variable() {
        let script = Script {
            actions: vec![
                ScriptAction::SetVariable {
                    name: "token".to_string(),
                    value: "abc123".to_string(),
                },
                ScriptAction::SetHeader {
                    key: "Authorization".to_string(),
                    value: "Bearer {{token}}".to_string(),
                },
            ],
        };
        let mut request = make_request();
        let mut context = ScriptContext::new();

        ScriptEngine::execute_pre_request(&script, &mut request, &mut context).unwrap();
        assert!(request
            .headers
            .iter()
            .any(|(k, v)| k == "Authorization" && v == "Bearer abc123"));
    }

    #[test]
    fn remove_header_pre_request() {
        let script = Script {
            actions: vec![ScriptAction::RemoveHeader {
                key: "X-Remove-Me".to_string(),
            }],
        };
        let mut request = make_request();
        request
            .headers
            .push(("X-Remove-Me".to_string(), "value".to_string()));
        let mut context = ScriptContext::new();

        ScriptEngine::execute_pre_request(&script, &mut request, &mut context).unwrap();
        assert!(!request.headers.iter().any(|(k, _)| k == "X-Remove-Me"));
    }

    #[test]
    fn set_body_pre_request() {
        let script = Script {
            actions: vec![ScriptAction::SetBody {
                body: r#"{"key":"value"}"#.to_string(),
            }],
        };
        let mut request = make_request();
        let mut context = ScriptContext::new();

        ScriptEngine::execute_pre_request(&script, &mut request, &mut context).unwrap();
        assert_eq!(request.body.as_deref(), Some(r#"{"key":"value"}"#));
    }

    #[test]
    fn set_url_pre_request() {
        let script = Script {
            actions: vec![ScriptAction::SetUrl {
                url: "https://other.api.com/data".to_string(),
            }],
        };
        let mut request = make_request();
        let mut context = ScriptContext::new();

        ScriptEngine::execute_pre_request(&script, &mut request, &mut context).unwrap();
        assert_eq!(request.url, "https://other.api.com/data");
    }

    #[test]
    fn set_method_pre_request() {
        let script = Script {
            actions: vec![ScriptAction::SetMethod {
                method: "POST".to_string(),
            }],
        };
        let mut request = make_request();
        let mut context = ScriptContext::new();

        ScriptEngine::execute_pre_request(&script, &mut request, &mut context).unwrap();
        assert_eq!(request.method, HttpMethod::Post);
    }

    #[test]
    fn set_method_invalid_returns_error() {
        let script = Script {
            actions: vec![ScriptAction::SetMethod {
                method: "NOTVALID".to_string(),
            }],
        };
        let mut request = make_request();
        let mut context = ScriptContext::new();

        let result = ScriptEngine::execute_pre_request(&script, &mut request, &mut context);
        assert!(result.is_err());
    }

    #[test]
    fn assert_status_ok() {
        let script = Script {
            actions: vec![ScriptAction::AssertStatus { expected: 200 }],
        };
        let response = make_response(200, "{}");
        let mut context = ScriptContext::new();

        let result = ScriptEngine::execute_post_response(&script, &response, &mut context);
        assert!(result.is_ok());
        assert!(context.errors.is_empty());
    }

    #[test]
    fn assert_status_fail() {
        let script = Script {
            actions: vec![ScriptAction::AssertStatus { expected: 200 }],
        };
        let response = make_response(404, "{}");
        let mut context = ScriptContext::new();

        let result = ScriptEngine::execute_post_response(&script, &response, &mut context);
        assert!(result.is_err());
        assert_eq!(context.errors.len(), 1);
    }

    #[test]
    fn assert_header_contains() {
        let script = Script {
            actions: vec![ScriptAction::AssertHeader {
                key: "content-type".to_string(),
                contains: Some("json".to_string()),
                equals: None,
            }],
        };
        let response = make_response(200, "{}");
        let mut context = ScriptContext::new();

        let result = ScriptEngine::execute_post_response(&script, &response, &mut context);
        assert!(result.is_ok());
    }

    #[test]
    fn assert_header_contains_fail() {
        let script = Script {
            actions: vec![ScriptAction::AssertHeader {
                key: "content-type".to_string(),
                contains: Some("xml".to_string()),
                equals: None,
            }],
        };
        let response = make_response(200, "{}");
        let mut context = ScriptContext::new();

        let result = ScriptEngine::execute_post_response(&script, &response, &mut context);
        assert!(result.is_err());
    }

    #[test]
    fn assert_header_not_found() {
        let script = Script {
            actions: vec![ScriptAction::AssertHeader {
                key: "x-custom".to_string(),
                contains: None,
                equals: Some("value".to_string()),
            }],
        };
        let response = make_response(200, "{}");
        let mut context = ScriptContext::new();

        let result = ScriptEngine::execute_post_response(&script, &response, &mut context);
        assert!(result.is_err());
    }

    #[test]
    fn assert_body_contains() {
        let script = Script {
            actions: vec![ScriptAction::AssertBody {
                contains: Some("users".to_string()),
                equals: None,
            }],
        };
        let response = make_response(200, r#"{"users": []}"#);
        let mut context = ScriptContext::new();

        let result = ScriptEngine::execute_post_response(&script, &response, &mut context);
        assert!(result.is_ok());
    }

    #[test]
    fn assert_body_contains_fail() {
        let script = Script {
            actions: vec![ScriptAction::AssertBody {
                contains: Some("orders".to_string()),
                equals: None,
            }],
        };
        let response = make_response(200, r#"{"users": []}"#);
        let mut context = ScriptContext::new();

        let result = ScriptEngine::execute_post_response(&script, &response, &mut context);
        assert!(result.is_err());
    }

    #[test]
    fn extract_json_field() {
        let script = Script {
            actions: vec![ScriptAction::ExtractJson {
                variable: "user_id".to_string(),
                path: "data.id".to_string(),
            }],
        };
        let response = make_response(200, r#"{"data": {"id": 42, "name": "John"}}"#);
        let mut context = ScriptContext::new();

        ScriptEngine::execute_post_response(&script, &response, &mut context).unwrap();
        assert_eq!(context.variables.get("user_id").unwrap(), "42");
    }

    #[test]
    fn extract_header_value() {
        let script = Script {
            actions: vec![ScriptAction::ExtractHeader {
                variable: "content_type".to_string(),
                header: "content-type".to_string(),
            }],
        };
        let response = make_response(200, "{}");
        let mut context = ScriptContext::new();

        ScriptEngine::execute_post_response(&script, &response, &mut context).unwrap();
        assert_eq!(
            context.variables.get("content_type").unwrap(),
            "application/json"
        );
    }

    #[test]
    fn log_message() {
        let script = Script {
            actions: vec![ScriptAction::Log {
                message: "Request sent".to_string(),
            }],
        };
        let mut request = make_request();
        let mut context = ScriptContext::new();

        ScriptEngine::execute_pre_request(&script, &mut request, &mut context).unwrap();
        assert_eq!(context.logs.len(), 1);
        assert_eq!(context.logs[0], "Request sent");
    }

    #[test]
    fn log_with_variable() {
        let script = Script {
            actions: vec![
                ScriptAction::SetVariable {
                    name: "url".to_string(),
                    value: "https://api.example.com".to_string(),
                },
                ScriptAction::Log {
                    message: "Calling {{url}}".to_string(),
                },
            ],
        };
        let mut request = make_request();
        let mut context = ScriptContext::new();

        ScriptEngine::execute_pre_request(&script, &mut request, &mut context).unwrap();
        assert_eq!(context.logs[0], "Calling https://api.example.com");
    }

    #[test]
    fn context_resolve_variables() {
        let mut context = ScriptContext::new();
        context
            .variables
            .insert("host".to_string(), "api.example.com".to_string());
        context
            .variables
            .insert("version".to_string(), "v2".to_string());

        let resolved = context.resolve_variables("https://{{host}}/{{version}}/users");
        assert_eq!(resolved, "https://api.example.com/v2/users");
    }

    #[test]
    fn context_resolve_nested_variables() {
        let mut context = ScriptContext::new();
        context.variables.insert(
            "base_url".to_string(),
            "https://{{host}}/{{version}}".to_string(),
        );
        context
            .variables
            .insert("host".to_string(), "api.example.com".to_string());
        context
            .variables
            .insert("version".to_string(), "v2".to_string());

        let resolved = context.resolve_variables("{{base_url}}/users");
        assert_eq!(resolved, "https://api.example.com/v2/users");
    }

    #[test]
    fn context_resolve_unresolved_variable_warning() {
        let mut context = ScriptContext::new();
        context
            .variables
            .insert("name".to_string(), "Alice".to_string());

        let resolved = context.resolve_variables("Hello {{name}} and {{missing}}");
        assert_eq!(resolved, "Hello Alice and {{missing}}");
        assert!(context.errors.iter().any(|e| e.contains("missing")));
    }

    #[test]
    fn context_resolve_dynamic_timestamp() {
        let mut context = ScriptContext::new();
        let resolved = context.resolve_variables("{{$timestamp}}");
        assert!(resolved.parse::<i64>().is_ok());
    }

    #[test]
    fn context_resolve_dynamic_uuid() {
        let mut context = ScriptContext::new();
        let resolved = context.resolve_variables("{{$uuid}}");
        assert_eq!(resolved.len(), 36);
        assert_eq!(resolved.chars().nth(8), Some('-'));
        assert_eq!(resolved.chars().nth(13), Some('-'));
    }

    #[test]
    fn context_resolve_dynamic_random_int() {
        let mut context = ScriptContext::new();
        let resolved = context.resolve_variables("{{$randomInt}}");
        assert!(resolved.parse::<u32>().is_ok());
    }

    #[test]
    fn context_resolve_dynamic_iso_now() {
        let mut context = ScriptContext::new();
        let resolved = context.resolve_variables("{{$isoNow}}");
        assert!(resolved.contains('T'));
    }

    #[test]
    fn request_scripts_json_roundtrip() {
        let scripts = RequestScripts {
            pre_request: Script {
                actions: vec![ScriptAction::SetHeader {
                    key: "X-Request-Time".to_string(),
                    value: "12345".to_string(),
                }],
            },
            post_response: Script {
                actions: vec![ScriptAction::AssertStatus { expected: 200 }],
            },
            ..Default::default()
        };

        let json = scripts.to_json().unwrap();
        let restored = RequestScripts::from_json(&json).unwrap();
        assert_eq!(scripts, restored);
    }

    #[test]
    fn request_scripts_from_empty_json() {
        let scripts = RequestScripts::from_json("").unwrap();
        assert!(scripts.pre_request.actions.is_empty());
        assert!(scripts.post_response.actions.is_empty());
    }

    #[test]
    fn multiple_pre_request_actions() {
        let script = Script {
            actions: vec![
                ScriptAction::SetVariable {
                    name: "token".to_string(),
                    value: "abc123".to_string(),
                },
                ScriptAction::SetHeader {
                    key: "Authorization".to_string(),
                    value: "Bearer {{token}}".to_string(),
                },
                ScriptAction::SetHeader {
                    key: "X-Custom".to_string(),
                    value: "fixed-value".to_string(),
                },
                ScriptAction::SetBody {
                    body: r#"{"token":"{{token}}"}"#.to_string(),
                },
            ],
        };
        let mut request = make_request();
        let mut context = ScriptContext::new();

        ScriptEngine::execute_pre_request(&script, &mut request, &mut context).unwrap();

        assert_eq!(context.variables.get("token").unwrap(), "abc123");
        assert!(request
            .headers
            .iter()
            .any(|(k, v)| k == "Authorization" && v == "Bearer abc123"));
        assert!(request
            .headers
            .iter()
            .any(|(k, v)| k == "X-Custom" && v == "fixed-value"));
        assert_eq!(request.body.as_deref(), Some(r#"{"token":"abc123"}"#));
    }

    #[test]
    fn post_response_chained_extractions() {
        let script = Script {
            actions: vec![
                ScriptAction::ExtractJson {
                    variable: "user_id".to_string(),
                    path: "data.id".to_string(),
                },
                ScriptAction::Log {
                    message: "Extracted user_id={{user_id}}".to_string(),
                },
            ],
        };
        let response = make_response(200, r#"{"data": {"id": 42}}"#);
        let mut context = ScriptContext::new();

        ScriptEngine::execute_post_response(&script, &response, &mut context).unwrap();
        assert_eq!(context.variables.get("user_id").unwrap(), "42");
        assert_eq!(context.logs[0], "Extracted user_id=42");
    }

    #[test]
    fn extract_json_nested_array() {
        let script = Script {
            actions: vec![ScriptAction::ExtractJson {
                variable: "first_name".to_string(),
                path: "users.0.name".to_string(),
            }],
        };
        let response = make_response(200, r#"{"users": [{"name": "Alice"}, {"name": "Bob"}]}"#);
        let mut context = ScriptContext::new();

        ScriptEngine::execute_post_response(&script, &response, &mut context).unwrap();
        assert_eq!(context.variables.get("first_name").unwrap(), "Alice");
    }

    #[test]
    fn extract_json_nonexistent_path() {
        let script = Script {
            actions: vec![ScriptAction::ExtractJson {
                variable: "missing".to_string(),
                path: "data.nonexistent".to_string(),
            }],
        };
        let response = make_response(200, r#"{"data": {"id": 1}}"#);
        let mut context = ScriptContext::new();

        ScriptEngine::execute_post_response(&script, &response, &mut context).unwrap();
        assert!(!context.variables.contains_key("missing"));
        assert_eq!(context.errors.len(), 1);
    }

    #[test]
    fn assert_status_on_error_response() {
        let script = Script {
            actions: vec![ScriptAction::AssertStatus { expected: 200 }],
        };
        let response = make_response(500, "Internal Server Error");
        let mut context = ScriptContext::new();

        let result = ScriptEngine::execute_post_response(&script, &response, &mut context);
        assert!(result.is_err());
        assert!(context.errors[0].contains("500"));
    }

    #[test]
    fn script_action_display() {
        assert_eq!(
            ScriptAction::SetVariable {
                name: "x".to_string(),
                value: "1".to_string()
            }
            .to_string(),
            "set_var(x=1)"
        );
        assert_eq!(
            ScriptAction::SetHeader {
                key: "Auth".to_string(),
                value: "Bearer token".to_string()
            }
            .to_string(),
            "set_header(Auth: Bearer token)"
        );
        assert_eq!(
            ScriptAction::RemoveHeader {
                key: "X-Remove".to_string()
            }
            .to_string(),
            "remove_header(X-Remove)"
        );
        assert_eq!(
            ScriptAction::AssertStatus { expected: 200 }.to_string(),
            "assert_status(200)"
        );
        assert_eq!(
            ScriptAction::Log {
                message: "hello".to_string()
            }
            .to_string(),
            "log(hello)"
        );
        assert_eq!(ScriptAction::Delay { ms: 100 }.to_string(), "delay(100ms)");
        assert_eq!(
            ScriptAction::ExtractJson {
                variable: "id".to_string(),
                path: "data.id".to_string()
            }
            .to_string(),
            "extract_json(id from data.id)"
        );
        assert_eq!(
            ScriptAction::ExtractHeader {
                variable: "ct".to_string(),
                header: "content-type".to_string()
            }
            .to_string(),
            "extract_header(ct from content-type)"
        );
    }

    #[test]
    fn extract_json_bracket_notation() {
        let script = Script {
            actions: vec![ScriptAction::ExtractJson {
                variable: "first_id".to_string(),
                path: "items[0].id".to_string(),
            }],
        };
        let response = make_response(
            200,
            r#"{"items": [{"id": 101, "name": "Item A"}, {"id": 102, "name": "Item B"}]}"#,
        );
        let mut context = ScriptContext::new();

        ScriptEngine::execute_post_response(&script, &response, &mut context).unwrap();
        assert_eq!(context.variables.get("first_id").unwrap(), "101");
    }

    #[test]
    fn extract_json_deeply_nested_bracket() {
        let script = Script {
            actions: vec![ScriptAction::ExtractJson {
                variable: "city".to_string(),
                path: "data.users[0].addresses[0].city".to_string(),
            }],
        };
        let response = make_response(
            200,
            r#"{"data": {"users": [{"addresses": [{"city": "Madrid"}]}]}}"#,
        );
        let mut context = ScriptContext::new();

        ScriptEngine::execute_post_response(&script, &response, &mut context).unwrap();
        assert_eq!(context.variables.get("city").unwrap(), "Madrid");
    }

    #[test]
    fn extract_json_multiple_brackets() {
        let script = Script {
            actions: vec![ScriptAction::ExtractJson {
                variable: "val".to_string(),
                path: "matrix[1][2]".to_string(),
            }],
        };
        let response = make_response(200, r#"{"matrix": [[1,2,3],[4,5,6],[7,8,9]]}"#);
        let mut context = ScriptContext::new();

        ScriptEngine::execute_post_response(&script, &response, &mut context).unwrap();
        assert_eq!(context.variables.get("val").unwrap(), "6");
    }

    #[test]
    fn post_response_accumulates_all_errors() {
        let script = Script {
            actions: vec![
                ScriptAction::AssertStatus { expected: 200 },
                ScriptAction::AssertHeader {
                    key: "x-missing".to_string(),
                    contains: None,
                    equals: Some("nope".to_string()),
                },
                ScriptAction::ExtractJson {
                    variable: "data".to_string(),
                    path: "missing.path".to_string(),
                },
            ],
        };
        let response = make_response(404, r#"{"other": true}"#);
        let mut context = ScriptContext::new();

        let result = ScriptEngine::execute_post_response(&script, &response, &mut context);
        assert!(result.is_err());
        assert!(context.errors.len() >= 3);
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("404"));
        assert!(err_msg.contains("x-missing"));
    }

    #[test]
    fn post_response_extracts_even_after_assert_fail() {
        let script = Script {
            actions: vec![
                ScriptAction::AssertStatus { expected: 200 },
                ScriptAction::ExtractJson {
                    variable: "error_code".to_string(),
                    path: "error.code".to_string(),
                },
            ],
        };
        let response = make_response(400, r#"{"error": {"code": "INVALID_INPUT"}}"#);
        let mut context = ScriptContext::new();

        let result = ScriptEngine::execute_post_response(&script, &response, &mut context);
        assert!(result.is_err());
        assert_eq!(
            context.variables.get("error_code").unwrap(),
            "INVALID_INPUT"
        );
    }

    #[test]
    fn transform_to_upper() {
        let script = Script {
            actions: vec![ScriptAction::TransformToUpper {
                input: "hello".to_string(),
                variable: "upper".to_string(),
            }],
        };
        let mut request = make_request();
        let mut context = ScriptContext::new();

        ScriptEngine::execute_pre_request(&script, &mut request, &mut context).unwrap();
        assert_eq!(context.variables.get("upper").unwrap(), "HELLO");
    }

    #[test]
    fn transform_to_lower() {
        let script = Script {
            actions: vec![ScriptAction::TransformToLower {
                input: "HELLO".to_string(),
                variable: "lower".to_string(),
            }],
        };
        let mut request = make_request();
        let mut context = ScriptContext::new();

        ScriptEngine::execute_pre_request(&script, &mut request, &mut context).unwrap();
        assert_eq!(context.variables.get("lower").unwrap(), "hello");
    }

    #[test]
    fn transform_trim() {
        let script = Script {
            actions: vec![ScriptAction::TransformTrim {
                input: "  hello  ".to_string(),
                variable: "trimmed".to_string(),
            }],
        };
        let mut request = make_request();
        let mut context = ScriptContext::new();

        ScriptEngine::execute_pre_request(&script, &mut request, &mut context).unwrap();
        assert_eq!(context.variables.get("trimmed").unwrap(), "hello");
    }

    #[test]
    fn encode_base64() {
        let script = Script {
            actions: vec![ScriptAction::EncodeBase64 {
                input: "hello world".to_string(),
                variable: "encoded".to_string(),
            }],
        };
        let mut request = make_request();
        let mut context = ScriptContext::new();

        ScriptEngine::execute_pre_request(&script, &mut request, &mut context).unwrap();
        assert_eq!(
            context.variables.get("encoded").unwrap(),
            "aGVsbG8gd29ybGQ="
        );
    }

    #[test]
    fn decode_base64() {
        let script = Script {
            actions: vec![ScriptAction::DecodeBase64 {
                input: "aGVsbG8gd29ybGQ=".to_string(),
                variable: "decoded".to_string(),
            }],
        };
        let mut request = make_request();
        let mut context = ScriptContext::new();

        ScriptEngine::execute_pre_request(&script, &mut request, &mut context).unwrap();
        assert_eq!(context.variables.get("decoded").unwrap(), "hello world");
    }

    #[test]
    fn decode_base64_invalid() {
        let script = Script {
            actions: vec![ScriptAction::DecodeBase64 {
                input: "not-valid-base64!!!".to_string(),
                variable: "decoded".to_string(),
            }],
        };
        let mut request = make_request();
        let mut context = ScriptContext::new();

        let result = ScriptEngine::execute_pre_request(&script, &mut request, &mut context);
        assert!(result.is_err());
    }

    #[test]
    fn hash_sha256() {
        let script = Script {
            actions: vec![ScriptAction::HashSha256 {
                input: "hello".to_string(),
                variable: "hash".to_string(),
            }],
        };
        let mut request = make_request();
        let mut context = ScriptContext::new();

        ScriptEngine::execute_pre_request(&script, &mut request, &mut context).unwrap();
        let hash = context.variables.get("hash").unwrap();
        assert_eq!(hash.len(), 64);
        assert_eq!(
            hash,
            "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
        );
    }

    #[test]
    fn hmac_sha256() {
        let script = Script {
            actions: vec![ScriptAction::HmacSha256 {
                key: "secret".to_string(),
                message: "hello".to_string(),
                variable: "signature".to_string(),
            }],
        };
        let mut request = make_request();
        let mut context = ScriptContext::new();

        ScriptEngine::execute_pre_request(&script, &mut request, &mut context).unwrap();
        let sig = context.variables.get("signature").unwrap();
        assert_eq!(sig.len(), 64);
    }

    #[test]
    fn hmac_sha256_with_variable_key() {
        let script = Script {
            actions: vec![
                ScriptAction::SetVariable {
                    name: "api_key".to_string(),
                    value: "my-secret-key".to_string(),
                },
                ScriptAction::HmacSha256 {
                    key: "{{api_key}}".to_string(),
                    message: "payload".to_string(),
                    variable: "sig".to_string(),
                },
            ],
        };
        let mut request = make_request();
        let mut context = ScriptContext::new();

        ScriptEngine::execute_pre_request(&script, &mut request, &mut context).unwrap();
        let sig = context.variables.get("sig").unwrap();
        assert_eq!(sig.len(), 64);
    }

    #[test]
    fn if_status_then_branch() {
        let script = Script {
            actions: vec![ScriptAction::IfStatus {
                code: 200,
                then: vec![
                    ScriptAction::ExtractJson {
                        variable: "token".to_string(),
                        path: "data.token".to_string(),
                    },
                    ScriptAction::Log {
                        message: "Token extracted".to_string(),
                    },
                ],
                else_actions: Some(vec![ScriptAction::Log {
                    message: "Failed".to_string(),
                }]),
            }],
        };
        let response = make_response(200, r#"{"data": {"token": "abc123"}}"#);
        let mut context = ScriptContext::new();

        ScriptEngine::execute_post_response(&script, &response, &mut context).unwrap();
        assert_eq!(context.variables.get("token").unwrap(), "abc123");
        assert_eq!(context.logs.len(), 1);
        assert_eq!(context.logs[0], "Token extracted");
    }

    #[test]
    fn if_status_else_branch() {
        let script = Script {
            actions: vec![ScriptAction::IfStatus {
                code: 200,
                then: vec![ScriptAction::Log {
                    message: "Success".to_string(),
                }],
                else_actions: Some(vec![ScriptAction::Log {
                    message: "Failed".to_string(),
                }]),
            }],
        };
        let response = make_response(500, "{}");
        let mut context = ScriptContext::new();

        ScriptEngine::execute_post_response(&script, &response, &mut context).unwrap();
        assert_eq!(context.logs.len(), 1);
        assert_eq!(context.logs[0], "Failed");
    }

    #[test]
    fn if_status_no_else() {
        let script = Script {
            actions: vec![ScriptAction::IfStatus {
                code: 200,
                then: vec![ScriptAction::Log {
                    message: "Success".to_string(),
                }],
                else_actions: None,
            }],
        };
        let response = make_response(404, "{}");
        let mut context = ScriptContext::new();

        ScriptEngine::execute_post_response(&script, &response, &mut context).unwrap();
        assert!(context.logs.is_empty());
    }

    #[test]
    fn if_status_nested_actions_continue_on_error() {
        let script = Script {
            actions: vec![ScriptAction::IfStatus {
                code: 200,
                then: vec![
                    ScriptAction::AssertStatus { expected: 200 },
                    ScriptAction::Log {
                        message: "This runs even if assert above fails".to_string(),
                    },
                ],
                else_actions: None,
            }],
        };
        let response = make_response(200, "{}");
        let mut context = ScriptContext::new();

        ScriptEngine::execute_post_response(&script, &response, &mut context).unwrap();
        assert_eq!(context.logs.len(), 1);
    }

    #[test]
    fn parse_json_path_segments_simple() {
        let segments = parse_json_path_segments("data.id");
        assert_eq!(segments.len(), 2);
        assert!(matches!(&segments[0], PathSegment::Key(k) if k == "data"));
        assert!(matches!(&segments[1], PathSegment::Key(k) if k == "id"));
    }

    #[test]
    fn parse_json_path_segments_bracket() {
        let segments = parse_json_path_segments("items[0].id");
        assert_eq!(segments.len(), 3);
        assert!(matches!(&segments[0], PathSegment::Key(k) if k == "items"));
        assert!(matches!(&segments[1], PathSegment::Index(i) if *i == 0));
        assert!(matches!(&segments[2], PathSegment::Key(k) if k == "id"));
    }

    #[test]
    fn parse_json_path_segments_multiple_brackets() {
        let segments = parse_json_path_segments("matrix[1][2]");
        assert_eq!(segments.len(), 3);
        assert!(matches!(&segments[0], PathSegment::Key(k) if k == "matrix"));
        assert!(matches!(&segments[1], PathSegment::Index(i) if *i == 1));
        assert!(matches!(&segments[2], PathSegment::Index(i) if *i == 2));
    }

    #[test]
    fn parse_json_path_segments_numeric_start() {
        let segments = parse_json_path_segments("0.name");
        assert_eq!(segments.len(), 2);
        assert!(matches!(&segments[0], PathSegment::Index(i) if *i == 0));
        assert!(matches!(&segments[1], PathSegment::Key(k) if k == "name"));
    }

    #[test]
    fn generate_uuid_v4_format() {
        let uuid = generate_uuid_v4();
        assert_eq!(uuid.len(), 36);
        assert_eq!(uuid.chars().nth(8), Some('-'));
        assert_eq!(uuid.chars().nth(13), Some('-'));
        assert_eq!(uuid.chars().nth(18), Some('-'));
        assert_eq!(uuid.chars().nth(23), Some('-'));
    }

    #[test]
    fn transform_to_upper_with_variable() {
        let script = Script {
            actions: vec![
                ScriptAction::SetVariable {
                    name: "name".to_string(),
                    value: "alice".to_string(),
                },
                ScriptAction::TransformToUpper {
                    input: "{{name}}".to_string(),
                    variable: "upper_name".to_string(),
                },
            ],
        };
        let mut request = make_request();
        let mut context = ScriptContext::new();

        ScriptEngine::execute_pre_request(&script, &mut request, &mut context).unwrap();
        assert_eq!(context.variables.get("upper_name").unwrap(), "ALICE");
    }

    #[test]
    fn encode_base64_with_variable() {
        let script = Script {
            actions: vec![
                ScriptAction::SetVariable {
                    name: "data".to_string(),
                    value: "secret".to_string(),
                },
                ScriptAction::EncodeBase64 {
                    input: "{{data}}".to_string(),
                    variable: "encoded".to_string(),
                },
            ],
        };
        let mut request = make_request();
        let mut context = ScriptContext::new();

        ScriptEngine::execute_pre_request(&script, &mut request, &mut context).unwrap();
        assert_eq!(
            context.variables.get("encoded").unwrap().as_str(),
            base64::engine::general_purpose::STANDARD
                .encode("secret")
                .as_str()
        );
    }

    #[test]
    fn extract_json_with_variable_path() {
        let script = Script {
            actions: vec![
                ScriptAction::SetVariable {
                    name: "idx".to_string(),
                    value: "1".to_string(),
                },
                ScriptAction::ExtractJson {
                    variable: "name".to_string(),
                    path: "users[{{idx}}].name".to_string(),
                },
            ],
        };
        let response = make_response(200, r#"{"users": [{"name": "Alice"}, {"name": "Bob"}]}"#);
        let mut context = ScriptContext::new();

        ScriptEngine::execute_post_response(&script, &response, &mut context).unwrap();
        assert_eq!(context.variables.get("name").unwrap(), "Bob");
    }

    #[test]
    fn extract_regex_from_body() {
        let script = Script {
            actions: vec![ScriptAction::ExtractRegex {
                variable: "token".to_string(),
                pattern: r#""access_token":"([^"]+)""#.to_string(),
            }],
        };
        let response = make_response(200, r#"{"access_token":"abc123xyz","type":"Bearer"}"#);
        let mut context = ScriptContext::new();

        ScriptEngine::execute_post_response(&script, &response, &mut context).unwrap();
        assert_eq!(context.variables.get("token").unwrap(), "abc123xyz");
    }

    #[test]
    fn extract_regex_no_match() {
        let script = Script {
            actions: vec![ScriptAction::ExtractRegex {
                variable: "token".to_string(),
                pattern: r#""missing":"([^"]+)""#.to_string(),
            }],
        };
        let response = make_response(200, r#"{"other":"value"}"#);
        let mut context = ScriptContext::new();

        ScriptEngine::execute_post_response(&script, &response, &mut context).unwrap();
        assert!(!context.variables.contains_key("token"));
        assert_eq!(context.errors.len(), 1);
    }

    #[test]
    fn extract_regex_logs_error_on_invalid_pattern() {
        let script = Script {
            actions: vec![ScriptAction::ExtractRegex {
                variable: "token".to_string(),
                pattern: "(".to_string(),
            }],
        };
        let response = make_response(200, "{}");
        let mut context = ScriptContext::new();

        let result = ScriptEngine::execute_post_response(&script, &response, &mut context);
        assert!(result.is_err());
        assert!(context.errors.iter().any(|e| e.contains("regex")));
    }

    #[test]
    fn extract_regex_full_match_fallback() {
        let script = Script {
            actions: vec![ScriptAction::ExtractRegex {
                variable: "version".to_string(),
                pattern: r"v\d+\.\d+\.\d+".to_string(),
            }],
        };
        let response = make_response(200, "API version v2.1.0 is current");
        let mut context = ScriptContext::new();

        ScriptEngine::execute_post_response(&script, &response, &mut context).unwrap();
        assert_eq!(context.variables.get("version").unwrap(), "v2.1.0");
    }

    #[test]
    fn set_body_json_merge() {
        let script = Script {
            actions: vec![
                ScriptAction::SetBody {
                    body: r#"{"name":"John","age":30}"#.to_string(),
                },
                ScriptAction::SetBodyJson {
                    path: "age".to_string(),
                    value: "31".to_string(),
                },
            ],
        };
        let mut request = make_request();
        let mut context = ScriptContext::new();

        ScriptEngine::execute_pre_request(&script, &mut request, &mut context).unwrap();
        let body: serde_json::Value =
            serde_json::from_str(request.body.as_deref().unwrap()).unwrap();
        assert_eq!(body["name"], "John");
        assert_eq!(body["age"], 31);
    }

    #[test]
    fn set_body_json_nested() {
        let script = Script {
            actions: vec![
                ScriptAction::SetBody {
                    body: r#"{"user":{"name":"Alice","address":{"city":"Madrid"}}}"#.to_string(),
                },
                ScriptAction::SetBodyJson {
                    path: "user.address.city".to_string(),
                    value: r#""Barcelona""#.to_string(),
                },
            ],
        };
        let mut request = make_request();
        let mut context = ScriptContext::new();

        ScriptEngine::execute_pre_request(&script, &mut request, &mut context).unwrap();
        let body: serde_json::Value =
            serde_json::from_str(request.body.as_deref().unwrap()).unwrap();
        assert_eq!(body["user"]["address"]["city"], "Barcelona");
    }

    #[test]
    fn set_body_json_creates_missing() {
        let script = Script {
            actions: vec![ScriptAction::SetBodyJson {
                path: "new_field".to_string(),
                value: r#""hello""#.to_string(),
            }],
        };
        let mut request = make_request();
        let mut context = ScriptContext::new();

        ScriptEngine::execute_pre_request(&script, &mut request, &mut context).unwrap();
        let body: serde_json::Value =
            serde_json::from_str(request.body.as_deref().unwrap()).unwrap();
        assert_eq!(body["new_field"], "hello");
    }

    #[test]
    fn set_query_adds_param() {
        let script = Script {
            actions: vec![ScriptAction::SetQuery {
                key: "page".to_string(),
                value: "2".to_string(),
            }],
        };
        let mut request = make_request();
        request.url = "https://api.example.com/users".to_string();
        let mut context = ScriptContext::new();

        ScriptEngine::execute_pre_request(&script, &mut request, &mut context).unwrap();
        assert!(request.url.contains("page=2"));
        assert!(request.url.contains("?"));
    }

    #[test]
    fn set_query_appends_to_existing() {
        let script = Script {
            actions: vec![ScriptAction::SetQuery {
                key: "sort".to_string(),
                value: "name".to_string(),
            }],
        };
        let mut request = make_request();
        request.url = "https://api.example.com/users?page=1".to_string();
        let mut context = ScriptContext::new();

        ScriptEngine::execute_pre_request(&script, &mut request, &mut context).unwrap();
        assert!(request.url.contains("page=1"));
        assert!(request.url.contains("sort=name"));
    }

    #[test]
    fn set_query_with_variable() {
        let script = Script {
            actions: vec![
                ScriptAction::SetVariable {
                    name: "token".to_string(),
                    value: "abc123".to_string(),
                },
                ScriptAction::SetQuery {
                    key: "access_token".to_string(),
                    value: "{{token}}".to_string(),
                },
            ],
        };
        let mut request = make_request();
        request.url = "https://api.example.com/data".to_string();
        let mut context = ScriptContext::new();

        ScriptEngine::execute_pre_request(&script, &mut request, &mut context).unwrap();
        assert!(request.url.contains("access_token=abc123"));
    }

    #[test]
    fn assert_json_path_equals() {
        let script = Script {
            actions: vec![ScriptAction::AssertJsonPath {
                path: "status".to_string(),
                equals: Some("active".to_string()),
                contains: None,
            }],
        };
        let response = make_response(200, r#"{"status":"active","count":5}"#);
        let mut context = ScriptContext::new();

        let result = ScriptEngine::execute_post_response(&script, &response, &mut context);
        assert!(result.is_ok());
    }

    #[test]
    fn assert_json_path_equals_fail() {
        let script = Script {
            actions: vec![ScriptAction::AssertJsonPath {
                path: "status".to_string(),
                equals: Some("active".to_string()),
                contains: None,
            }],
        };
        let response = make_response(200, r#"{"status":"inactive"}"#);
        let mut context = ScriptContext::new();

        let result = ScriptEngine::execute_post_response(&script, &response, &mut context);
        assert!(result.is_err());
    }

    #[test]
    fn assert_json_path_contains() {
        let script = Script {
            actions: vec![ScriptAction::AssertJsonPath {
                path: "message".to_string(),
                equals: None,
                contains: Some("success".to_string()),
            }],
        };
        let response = make_response(200, r#"{"message":"Operation completed successfully"}"#);
        let mut context = ScriptContext::new();

        let result = ScriptEngine::execute_post_response(&script, &response, &mut context);
        assert!(result.is_ok());
    }

    #[test]
    fn assert_json_path_not_found() {
        let script = Script {
            actions: vec![ScriptAction::AssertJsonPath {
                path: "missing.path".to_string(),
                equals: Some("value".to_string()),
                contains: None,
            }],
        };
        let response = make_response(200, r#"{"other":true}"#);
        let mut context = ScriptContext::new();

        let result = ScriptEngine::execute_post_response(&script, &response, &mut context);
        assert!(result.is_err());
    }

    #[test]
    fn assert_json_path_nested() {
        let script = Script {
            actions: vec![ScriptAction::AssertJsonPath {
                path: "data.user.name".to_string(),
                equals: Some("Alice".to_string()),
                contains: None,
            }],
        };
        let response = make_response(200, r#"{"data":{"user":{"name":"Alice"}}}"#);
        let mut context = ScriptContext::new();

        let result = ScriptEngine::execute_post_response(&script, &response, &mut context);
        assert!(result.is_ok());
    }

    #[test]
    fn set_query_invalid_url_fallback() {
        let script = Script {
            actions: vec![ScriptAction::SetQuery {
                key: "key".to_string(),
                value: "val".to_string(),
            }],
        };
        let mut request = make_request();
        request.url = "not-a-url".to_string();
        let mut context = ScriptContext::new();

        ScriptEngine::execute_pre_request(&script, &mut request, &mut context).unwrap();
        assert!(request.url.contains("key=val"));
    }
}
