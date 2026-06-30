use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenApiSpec {
    pub openapi: Option<String>,
    pub swagger: Option<String>,
    pub info: Info,
    #[serde(default)]
    pub servers: Vec<Server>,
    pub paths: HashMap<String, PathItem>,
    #[serde(default)]
    pub components: Option<Components>,
    #[serde(default)]
    pub definitions: Option<HashMap<String, Schema>>,
    #[serde(default)]
    pub tags: Vec<Tag>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Info {
    pub title: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Server {
    pub url: String,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tag {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathItem {
    #[serde(default)]
    pub get: Option<Operation>,
    #[serde(default)]
    pub post: Option<Operation>,
    #[serde(default)]
    pub put: Option<Operation>,
    #[serde(default)]
    pub patch: Option<Operation>,
    #[serde(default)]
    pub delete: Option<Operation>,
    #[serde(default)]
    pub head: Option<Operation>,
    #[serde(default)]
    pub options: Option<Operation>,
    #[serde(default)]
    pub parameters: Vec<Parameter>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Operation {
    #[serde(default)]
    pub summary: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default, rename = "operationId")]
    pub operation_id: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub parameters: Vec<Parameter>,
    #[serde(default, rename = "requestBody")]
    pub request_body: Option<RequestBody>,
    #[serde(default)]
    pub responses: HashMap<String, Response>,
    #[serde(default)]
    pub deprecated: bool,
    #[serde(default)]
    pub security: Vec<HashMap<String, Vec<String>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Parameter {
    pub name: String,
    #[serde(rename = "in")]
    pub location: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub schema: Option<Schema>,
    #[serde(default)]
    #[serde(rename = "type")]
    pub param_type: Option<String>,
    #[serde(default)]
    pub example: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestBody {
    #[serde(default)]
    pub description: Option<String>,
    pub content: HashMap<String, MediaType>,
    #[serde(default)]
    pub required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaType {
    #[serde(default)]
    pub schema: Option<Schema>,
    #[serde(default)]
    pub example: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Response {
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub content: HashMap<String, MediaType>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Components {
    #[serde(default)]
    pub schemas: Option<HashMap<String, Schema>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Schema {
    #[serde(default, rename = "type")]
    pub schema_type: Option<String>,
    #[serde(default)]
    pub format: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub properties: Option<HashMap<String, Schema>>,
    #[serde(default)]
    pub items: Option<Box<Schema>>,
    #[serde(default)]
    pub required: Vec<String>,
    #[serde(default)]
    pub example: Option<serde_json::Value>,
    #[serde(default, rename = "$ref")]
    pub r#ref: Option<String>,
    #[serde(default, rename = "allOf")]
    pub all_of: Option<Vec<Schema>>,
    #[serde(default, rename = "oneOf")]
    pub one_of: Option<Vec<Schema>>,
    #[serde(default, rename = "anyOf")]
    pub any_of: Option<Vec<Schema>>,
    #[serde(default, rename = "enum")]
    pub enum_values: Option<Vec<serde_json::Value>>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ParsedEndpoint {
    pub path: String,
    pub method: String,
    pub operation_id: Option<String>,
    pub summary: Option<String>,
    pub description: Option<String>,
    pub tags: Vec<String>,
    pub parameters: Vec<ParsedParameter>,
    pub request_body_example: Option<String>,
    pub response_example: Option<String>,
    pub deprecated: bool,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ParsedParameter {
    pub name: String,
    pub location: String,
    pub required: bool,
    pub description: Option<String>,
    pub example: Option<String>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ParsedSpec {
    pub title: String,
    pub description: Option<String>,
    pub version: Option<String>,
    pub base_url: Option<String>,
    pub endpoints: Vec<ParsedEndpoint>,
    pub tags: Vec<Tag>,
}

#[allow(dead_code)]
impl ParsedSpec {
    pub fn endpoints_by_tag(&self) -> HashMap<String, Vec<&ParsedEndpoint>> {
        let mut map: HashMap<String, Vec<&ParsedEndpoint>> = HashMap::new();
        for endpoint in &self.endpoints {
            let tag = endpoint
                .tags
                .first()
                .cloned()
                .unwrap_or_else(|| "default".to_string());
            map.entry(tag).or_default().push(endpoint);
        }
        map
    }

    pub fn search_endpoints(&self, query: &str) -> Vec<&ParsedEndpoint> {
        let query_lower = query.to_lowercase();
        self.endpoints
            .iter()
            .filter(|e| {
                e.path.to_lowercase().contains(&query_lower)
                    || e.summary
                        .as_ref()
                        .map(|s| s.to_lowercase().contains(&query_lower))
                        .unwrap_or(false)
                    || e.operation_id
                        .as_ref()
                        .map(|o| o.to_lowercase().contains(&query_lower))
                        .unwrap_or(false)
                    || e.method.to_lowercase().contains(&query_lower)
            })
            .collect()
    }
}
