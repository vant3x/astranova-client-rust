use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphQLRequest {
    pub query: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variables: Option<serde_json::Value>,
    #[serde(rename = "operationName", skip_serializing_if = "Option::is_none")]
    pub operation_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphQLResponse {
    pub data: Option<serde_json::Value>,
    #[serde(default)]
    pub errors: Vec<GraphQLError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphQLError {
    pub message: String,
    #[serde(default)]
    pub locations: Vec<GraphQLLocation>,
    #[serde(default)]
    pub path: Vec<GraphQLPathSegment>,
    #[serde(default)]
    pub extensions: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphQLLocation {
    pub line: u32,
    pub column: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum GraphQLPathSegment {
    String(String),
    Number(u32),
}

impl std::fmt::Display for GraphQLPathSegment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GraphQLPathSegment::String(s) => write!(f, "{}", s),
            GraphQLPathSegment::Number(n) => write!(f, "{}", n),
        }
    }
}

#[allow(dead_code)]
impl GraphQLRequest {
    pub fn new(query: &str) -> Self {
        Self {
            query: query.to_string(),
            variables: None,
            operation_name: None,
        }
    }

    pub fn with_variables(mut self, variables: serde_json::Value) -> Self {
        self.variables = Some(variables);
        self
    }

    pub fn with_operation_name(mut self, name: &str) -> Self {
        self.operation_name = Some(name.to_string());
        self
    }

    pub fn to_json(&self) -> Result<String, String> {
        serde_json::to_string_pretty(self).map_err(|e| e.to_string())
    }

    pub fn from_json(json: &str) -> Result<Self, String> {
        serde_json::from_str(json).map_err(|e| format!("Invalid JSON: {}", e))
    }
}

pub fn parse_variables(json_str: &str) -> Result<serde_json::Value, String> {
    if json_str.trim().is_empty() {
        return Ok(serde_json::Value::Null);
    }
    serde_json::from_str(json_str).map_err(|e| format!("Invalid variables JSON: {}", e))
}

pub fn validate_query(query: &str) -> Result<(), String> {
    let trimmed = query.trim();
    if trimmed.is_empty() {
        return Err("Query cannot be empty".to_string());
    }

    let has_operation = trimmed.starts_with("query")
        || trimmed.starts_with("mutation")
        || trimmed.starts_with("subscription")
        || trimmed.starts_with("fragment")
        || trimmed.starts_with("{");

    if !has_operation {
        return Err("Query must start with a valid operation: query, mutation, subscription, fragment, or shorthand { }".to_string());
    }

    let open_braces = trimmed.bytes().filter(|b| *b == b'{').count();
    let close_braces = trimmed.bytes().filter(|b| *b == b'}').count();
    if open_braces != close_braces {
        return Err(format!(
            "Unbalanced braces: {} opening, {} closing",
            open_braces, close_braces
        ));
    }

    Ok(())
}

pub fn format_response(response: &GraphQLResponse) -> String {
    let mut parts = Vec::new();

    if let Some(data) = &response.data {
        let pretty = serde_json::to_string_pretty(data).unwrap_or_else(|_| data.to_string());
        parts.push(pretty);
    }

    if !response.errors.is_empty() {
        let errors_str: Vec<String> = response
            .errors
            .iter()
            .enumerate()
            .map(|(i, err)| {
                let mut msg = format!("Error {}: {}", i + 1, err.message);
                if !err.locations.is_empty() {
                    let locs: Vec<String> = err
                        .locations
                        .iter()
                        .map(|l| format!("line {} col {}", l.line, l.column))
                        .collect();
                    msg.push_str(&format!(" at {}", locs.join(", ")));
                }
                if !err.path.is_empty() {
                    let path: String = err
                        .path
                        .iter()
                        .map(|p| p.to_string())
                        .collect::<Vec<_>>()
                        .join(".");
                    msg.push_str(&format!(" (path: {})", path));
                }
                msg
            })
            .collect();
        parts.push(errors_str.join("\n"));
    }

    if parts.is_empty() {
        "Empty response".to_string()
    } else {
        parts.join("\n\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn graphql_request_new() {
        let req = GraphQLRequest::new("{ users { id name } }");
        assert_eq!(req.query, "{ users { id name } }");
        assert!(req.variables.is_none());
        assert!(req.operation_name.is_none());
    }

    #[test]
    fn graphql_request_with_variables() {
        let vars = serde_json::json!({"id": 1});
        let req = GraphQLRequest::new("{ user(id: $id) { name } }").with_variables(vars.clone());
        assert_eq!(req.variables, Some(vars));
    }

    #[test]
    fn graphql_request_with_operation_name() {
        let req =
            GraphQLRequest::new("query GetUser { user { id } }").with_operation_name("GetUser");
        assert_eq!(req.operation_name, Some("GetUser".to_string()));
    }

    #[test]
    fn graphql_request_serialization() {
        let req = GraphQLRequest {
            query: "{ users { id } }".to_string(),
            variables: Some(serde_json::json!({"limit": 10})),
            operation_name: None,
        };
        let json = req.to_json().unwrap();
        assert!(json.contains("query"));
        assert!(json.contains("variables"));
        assert!(!json.contains("operationName"));
    }

    #[test]
    fn graphql_request_deserialization() {
        let json = r#"{
            "query": "{ users { id } }",
            "variables": {"limit": 10},
            "operationName": "GetUsers"
        }"#;
        let req: GraphQLRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.query, "{ users { id } }");
        assert_eq!(req.operation_name, Some("GetUsers".to_string()));
    }

    #[test]
    fn parse_variables_valid() {
        let vars = parse_variables(r#"{"key": "value"}"#).unwrap();
        assert_eq!(vars["key"], "value");
    }

    #[test]
    fn parse_variables_empty() {
        let vars = parse_variables("").unwrap();
        assert_eq!(vars, serde_json::Value::Null);
    }

    #[test]
    fn parse_variables_invalid() {
        let result = parse_variables("not json");
        assert!(result.is_err());
    }

    #[test]
    fn validate_query_empty() {
        assert!(validate_query("").is_err());
    }

    #[test]
    fn validate_query_shorthand() {
        assert!(validate_query("{ user { id } }").is_ok());
    }

    #[test]
    fn validate_query_with_operation() {
        assert!(validate_query("query GetUser { user { id } }").is_ok());
    }

    #[test]
    fn validate_query_mutation() {
        assert!(validate_query(
            "mutation CreateUser($name: String!) { createUser(name: $name) { id } }"
        )
        .is_ok());
    }

    #[test]
    fn validate_query_unbalanced_braces() {
        assert!(validate_query("{ user { id }").is_err());
    }

    #[test]
    fn graphql_response_with_data() {
        let json = r#"{"data": {"user": {"id": 1, "name": "John"}}}"#;
        let resp: GraphQLResponse = serde_json::from_str(json).unwrap();
        assert!(resp.data.is_some());
        assert!(resp.errors.is_empty());
    }

    #[test]
    fn graphql_response_with_errors() {
        let json = r#"{
            "data": null,
            "errors": [{
                "message": "Cannot query field 'invalid' on type 'User'",
                "locations": [{"line": 1, "column": 9}],
                "path": ["user", "invalid"]
            }]
        }"#;
        let resp: GraphQLResponse = serde_json::from_str(json).unwrap();
        assert!(resp.data.is_none());
        assert_eq!(resp.errors.len(), 1);
        assert_eq!(
            resp.errors[0].message,
            "Cannot query field 'invalid' on type 'User'"
        );
        assert_eq!(resp.errors[0].locations[0].line, 1);
        assert_eq!(resp.errors[0].path.len(), 2);
    }

    #[test]
    fn format_response_with_data() {
        let resp = GraphQLResponse {
            data: Some(serde_json::json!({"user": {"id": 1}})),
            errors: vec![],
        };
        let formatted = format_response(&resp);
        assert!(formatted.contains("user"));
        assert!(formatted.contains("1"));
    }

    #[test]
    fn format_response_with_errors() {
        let resp = GraphQLResponse {
            data: None,
            errors: vec![GraphQLError {
                message: "Field not found".to_string(),
                locations: vec![GraphQLLocation { line: 1, column: 5 }],
                path: vec![GraphQLPathSegment::String("user".to_string())],
                extensions: None,
            }],
        };
        let formatted = format_response(&resp);
        assert!(formatted.contains("Error 1: Field not found"));
        assert!(formatted.contains("line 1 col 5"));
        assert!(formatted.contains("path: user"));
    }

    #[test]
    fn format_response_empty() {
        let resp = GraphQLResponse {
            data: None,
            errors: vec![],
        };
        let formatted = format_response(&resp);
        assert_eq!(formatted, "Empty response");
    }

    #[test]
    fn graphql_path_segment_display() {
        assert_eq!(
            GraphQLPathSegment::String("users".to_string()).to_string(),
            "users"
        );
        assert_eq!(GraphQLPathSegment::Number(0).to_string(), "0");
    }
}
