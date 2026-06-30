use super::models::*;
use std::collections::HashMap;

pub fn parse_spec(content: &str) -> Result<ParsedSpec, String> {
    let spec_value: serde_json::Value =
        serde_json::from_str(content).map_err(|e| format!("Invalid JSON: {}", e))?;

    if spec_value.get("openapi").is_some() {
        parse_openapi3(&spec_value)
    } else if spec_value.get("swagger").is_some() {
        parse_swagger2(&spec_value)
    } else {
        Err("Not a valid OpenAPI or Swagger spec: missing 'openapi' or 'swagger' field".to_string())
    }
}

pub fn parse_spec_from_yaml(content: &str) -> Result<ParsedSpec, String> {
    let spec_value: serde_json::Value =
        serde_yaml::from_str(content).map_err(|e| format!("Invalid YAML: {}", e))?;

    if spec_value.get("openapi").is_some() {
        parse_openapi3(&spec_value)
    } else if spec_value.get("swagger").is_some() {
        parse_swagger2(&spec_value)
    } else {
        Err("Not a valid OpenAPI or Swagger spec: missing 'openapi' or 'swagger' field".to_string())
    }
}

fn parse_openapi3(value: &serde_json::Value) -> Result<ParsedSpec, String> {
    let spec: OpenApiSpec =
        serde_json::from_value(value.clone()).map_err(|e| format!("Failed to parse spec: {}", e))?;

    let base_url = spec
        .servers
        .first()
        .map(|s| s.url.clone())
        .or_else(|| {
            spec.info
                .description
                .as_ref()
                .and_then(|_| Some("http://localhost".to_string()))
        });

    let schemas = spec.components.as_ref().and_then(|c| c.schemas.clone());
    let mut endpoints = Vec::new();

    for (path, path_item) in &spec.paths {
        let operations = extract_operations(path_item);
        for (method, op) in operations {
            let endpoint = build_endpoint(path, &method, op, &schemas);
            endpoints.push(endpoint);
        }
    }

    Ok(ParsedSpec {
        title: spec.info.title.clone(),
        description: spec.info.description.clone(),
        version: spec.info.version.clone(),
        base_url,
        endpoints,
        tags: spec.tags,
    })
}

fn parse_swagger2(value: &serde_json::Value) -> Result<ParsedSpec, String> {
    let spec: OpenApiSpec =
        serde_json::from_value(value.clone()).map_err(|e| format!("Failed to parse spec: {}", e))?;

    let base_url = value
        .get("host")
        .and_then(|h| h.as_str())
        .map(|host| {
            let scheme = value
                .get("schemes")
                .and_then(|s| s.as_array())
                .and_then(|arr| arr.first())
                .and_then(|s| s.as_str())
                .unwrap_or("https");
            let base_path = value
                .get("basePath")
                .and_then(|bp| bp.as_str())
                .unwrap_or("/");
            format!("{}://{}{}", scheme, host, base_path)
        });

    let mut endpoints = Vec::new();

    for (path, path_item) in &spec.paths {
        let operations = extract_operations(path_item);
        for (method, op) in operations {
            let endpoint = build_endpoint(path, &method, op, &spec.definitions);
            endpoints.push(endpoint);
        }
    }

    Ok(ParsedSpec {
        title: spec.info.title.clone(),
        description: spec.info.description.clone(),
        version: spec.info.version.clone(),
        base_url,
        endpoints,
        tags: spec.tags,
    })
}

fn extract_operations(path_item: &PathItem) -> Vec<(String, &Operation)> {
    let mut ops = Vec::new();
    if let Some(ref op) = path_item.get {
        ops.push(("GET".to_string(), op));
    }
    if let Some(ref op) = path_item.post {
        ops.push(("POST".to_string(), op));
    }
    if let Some(ref op) = path_item.put {
        ops.push(("PUT".to_string(), op));
    }
    if let Some(ref op) = path_item.patch {
        ops.push(("PATCH".to_string(), op));
    }
    if let Some(ref op) = path_item.delete {
        ops.push(("DELETE".to_string(), op));
    }
    if let Some(ref op) = path_item.head {
        ops.push(("HEAD".to_string(), op));
    }
    if let Some(ref op) = path_item.options {
        ops.push(("OPTIONS".to_string(), op));
    }
    ops
}

fn build_endpoint(
    path: &str,
    method: &str,
    operation: &Operation,
    schemas: &Option<HashMap<String, Schema>>,
) -> ParsedEndpoint {
    let parameters: Vec<ParsedParameter> = operation
        .parameters
        .iter()
        .map(|p| ParsedParameter {
            name: p.name.clone(),
            location: p.location.clone(),
            required: p.required,
            description: p.description.clone(),
            example: p.example.as_ref().and_then(|v| v.as_str().map(|s| s.to_string())),
        })
        .collect();

    let request_body_example = operation
        .request_body
        .as_ref()
        .and_then(|rb| rb.content.get("application/json"))
        .and_then(|mt| {
            mt.example
                .as_ref()
                .and_then(|v| serde_json::to_string_pretty(v).ok())
                .or_else(|| {
                    mt.schema
                        .as_ref()
                        .and_then(|s| generate_example_from_schema(s, schemas))
                })
        });

    let response_example = operation
        .responses
        .get("200")
        .or(operation.responses.get("201"))
        .or(operation.responses.values().next())
        .and_then(|resp| {
            resp.content
                .get("application/json")
                .and_then(|mt| {
                    mt.example
                        .as_ref()
                        .and_then(|v| serde_json::to_string_pretty(v).ok())
                        .or_else(|| {
                            mt.schema
                                .as_ref()
                                .and_then(|s| generate_example_from_schema(s, schemas))
                        })
                })
        });

    ParsedEndpoint {
        path: path.to_string(),
        method: method.to_uppercase(),
        operation_id: operation.operation_id.clone(),
        summary: operation.summary.clone(),
        description: operation.description.clone(),
        tags: operation.tags.clone(),
        parameters,
        request_body_example,
        response_example,
        deprecated: operation.deprecated,
    }
}

fn generate_example_from_schema(
    schema: &Schema,
    schemas: &Option<HashMap<String, Schema>>,
) -> Option<String> {
    if let Some(ref example) = schema.example {
        return serde_json::to_string_pretty(example).ok();
    }

    if let Some(ref r#ref) = schema.r#ref {
        let ref_name = r#ref.split('/').last()?;
        if let Some(schemas) = schemas {
            if let Some(resolved) = schemas.get(ref_name) {
                return generate_example_from_schema(resolved, &Some(schemas.clone()));
            }
        }
    }

    let value = match schema.schema_type.as_deref() {
        Some("string") => {
            let s = match schema.format.as_deref() {
                Some("email") => "user@example.com",
                Some("uri") | Some("url") => "https://example.com",
                Some("uuid") => "550e8400-e29b-41d4-a716-446655440000",
                Some("date") => "2024-01-01",
                Some("date-time") => "2024-01-01T00:00:00Z",
                _ => "string",
            };
            serde_json::Value::String(s.to_string())
        }
        Some("integer") | Some("number") => serde_json::Value::Number(0.into()),
        Some("boolean") => serde_json::Value::Bool(false),
        Some("array") => {
            if let Some(ref items) = schema.items {
                if let Some(item_example) = generate_example_from_schema(items, schemas) {
                    let parsed: serde_json::Value =
                        serde_json::from_str(&item_example).ok()?;
                    serde_json::Value::Array(vec![parsed])
                } else {
                    serde_json::Value::Array(vec![])
                }
            } else {
                serde_json::Value::Array(vec![])
            }
        }
        Some("object") => {
            let mut map = serde_json::Map::new();
            if let Some(ref properties) = schema.properties {
                for (key, prop_schema) in properties {
                    if let Some(ex) = generate_example_from_schema(prop_schema, schemas) {
                        let parsed: serde_json::Value = serde_json::from_str(&ex).ok()?;
                        map.insert(key.clone(), parsed);
                    }
                }
            }
            serde_json::Value::Object(map)
        }
        _ => return None,
    };

    serde_json::to_string_pretty(&value).ok()
}

pub fn detect_format(content: &str) -> SpecFormat {
    if content.trim_start().starts_with('{') {
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(content) {
            if value.get("openapi").is_some() {
                return SpecFormat::OpenApi3Json;
            }
            if value.get("swagger").is_some() {
                return SpecFormat::Swagger2Json;
            }
        }
    }

    if content.trim_start().starts_with("openapi:")
        || content.trim_start().starts_with("swagger:")
    {
        return if content.contains("openapi: 3") {
            SpecFormat::OpenApi3Yaml
        } else {
            SpecFormat::Swagger2Yaml
        };
    }

    SpecFormat::Unknown
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpecFormat {
    OpenApi3Json,
    OpenApi3Yaml,
    Swagger2Json,
    Swagger2Yaml,
    Unknown,
}

impl SpecFormat {
    pub fn is_valid(&self) -> bool {
        !matches!(self, SpecFormat::Unknown)
    }

    pub fn label(&self) -> &str {
        match self {
            SpecFormat::OpenApi3Json => "OpenAPI 3.x (JSON)",
            SpecFormat::OpenApi3Yaml => "OpenAPI 3.x (YAML)",
            SpecFormat::Swagger2Json => "Swagger 2.0 (JSON)",
            SpecFormat::Swagger2Yaml => "Swagger 2.0 (YAML)",
            SpecFormat::Unknown => "Unknown",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_openapi3_json() {
        let json = r#"{
            "openapi": "3.0.0",
            "info": { "title": "Test API", "version": "1.0" },
            "paths": {
                "/users": {
                    "get": {
                        "summary": "List users",
                        "operationId": "listUsers",
                        "responses": {
                            "200": { "description": "OK" }
                        }
                    }
                }
            }
        }"#;
        let spec = parse_spec(json).unwrap();
        assert_eq!(spec.title, "Test API");
        assert_eq!(spec.endpoints.len(), 1);
        assert_eq!(spec.endpoints[0].path, "/users");
        assert_eq!(spec.endpoints[0].method, "GET");
    }

    #[test]
    fn parse_swagger2_json() {
        let json = r#"{
            "swagger": "2.0",
            "info": { "title": "Pet Store", "version": "1.0" },
            "host": "petstore.example.com",
            "basePath": "/api",
            "schemes": ["https"],
            "paths": {
                "/pets": {
                    "get": {
                        "summary": "List pets",
                        "operationId": "listPets",
                        "responses": {
                            "200": { "description": "OK" }
                        }
                    },
                    "post": {
                        "summary": "Create pet",
                        "operationId": "createPet",
                        "responses": {
                            "201": { "description": "Created" }
                        }
                    }
                }
            }
        }"#;
        let spec = parse_spec(json).unwrap();
        assert_eq!(spec.title, "Pet Store");
        assert_eq!(spec.base_url, Some("https://petstore.example.com/api".to_string()));
        assert_eq!(spec.endpoints.len(), 2);
    }

    #[test]
    fn parse_openapi3_with_servers() {
        let json = r#"{
            "openapi": "3.0.0",
            "info": { "title": "API", "version": "1.0" },
            "servers": [
                { "url": "https://api.example.com/v1", "description": "Production" }
            ],
            "paths": {}
        }"#;
        let spec = parse_spec(json).unwrap();
        assert_eq!(spec.base_url, Some("https://api.example.com/v1".to_string()));
    }

    #[test]
    fn parse_openapi3_with_parameters() {
        let json = r#"{
            "openapi": "3.0.0",
            "info": { "title": "API", "version": "1.0" },
            "paths": {
                "/users/{id}": {
                    "get": {
                        "parameters": [
                            {
                                "name": "id",
                                "in": "path",
                                "required": true,
                                "schema": { "type": "integer" }
                            }
                        ],
                        "responses": { "200": { "description": "OK" } }
                    }
                }
            }
        }"#;
        let spec = parse_spec(json).unwrap();
        assert_eq!(spec.endpoints[0].parameters.len(), 1);
        assert_eq!(spec.endpoints[0].parameters[0].name, "id");
        assert!(spec.endpoints[0].parameters[0].required);
    }

    #[test]
    fn parse_openapi3_with_request_body() {
        let json = r#"{
            "openapi": "3.0.0",
            "info": { "title": "API", "version": "1.0" },
            "paths": {
                "/users": {
                    "post": {
                        "requestBody": {
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "type": "object",
                                        "properties": {
                                            "name": { "type": "string" },
                                            "email": { "type": "string", "format": "email" }
                                        }
                                    }
                                }
                            }
                        },
                        "responses": { "201": { "description": "Created" } }
                    }
                }
            }
        }"#;
        let spec = parse_spec(json).unwrap();
        assert!(spec.endpoints[0].request_body_example.is_some());
        let example = spec.endpoints[0].request_body_example.as_ref().unwrap();
        assert!(example.contains("name"));
        assert!(example.contains("email"));
    }

    #[test]
    fn parse_openapi3_with_tags() {
        let json = r#"{
            "openapi": "3.0.0",
            "info": { "title": "API", "version": "1.0" },
            "tags": [
                { "name": "users", "description": "User operations" }
            ],
            "paths": {
                "/users": {
                    "get": {
                        "tags": ["users"],
                        "responses": { "200": { "description": "OK" } }
                    }
                }
            }
        }"#;
        let spec = parse_spec(json).unwrap();
        assert_eq!(spec.tags.len(), 1);
        assert_eq!(spec.tags[0].name, "users");
        assert_eq!(spec.endpoints[0].tags, vec!["users"]);
    }

    #[test]
    fn parse_invalid_json_returns_error() {
        assert!(parse_spec("not json").is_err());
    }

    #[test]
    fn parse_missing_openapi_and_swagger_returns_error() {
        assert!(parse_spec(r#"{"info": {"title": "test"}}"#).is_err());
    }

    #[test]
    fn detect_format_json() {
        assert_eq!(
            detect_format(r#"{"openapi": "3.0.0", "info": {"title": "test"}}"#),
            SpecFormat::OpenApi3Json
        );
        assert_eq!(
            detect_format(r#"{"swagger": "2.0", "info": {"title": "test"}}"#),
            SpecFormat::Swagger2Json
        );
    }

    #[test]
    fn detect_format_yaml() {
        assert_eq!(
            detect_format("openapi: 3.0.0\ninfo:\n  title: test"),
            SpecFormat::OpenApi3Yaml
        );
        assert_eq!(
            detect_format("swagger: '2.0'\ninfo:\n  title: test"),
            SpecFormat::Swagger2Yaml
        );
    }

    #[test]
    fn detect_format_unknown() {
        assert_eq!(detect_format("random text"), SpecFormat::Unknown);
    }

    #[test]
    fn search_endpoints() {
        let json = r#"{
            "openapi": "3.0.0",
            "info": { "title": "API", "version": "1.0" },
            "paths": {
                "/users": {
                    "get": { "summary": "List users", "responses": { "200": { "description": "OK" } } }
                },
                "/posts": {
                    "get": { "summary": "List posts", "responses": { "200": { "description": "OK" } } }
                }
            }
        }"#;
        let spec = parse_spec(json).unwrap();
        let results = spec.search_endpoints("user");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].path, "/users");
    }

    #[test]
    fn endpoints_by_tag() {
        let json = r#"{
            "openapi": "3.0.0",
            "info": { "title": "API", "version": "1.0" },
            "paths": {
                "/users": {
                    "get": { "tags": ["users"], "responses": { "200": { "description": "OK" } } }
                },
                "/posts": {
                    "get": { "tags": ["posts"], "responses": { "200": { "description": "OK" } } }
                }
            }
        }"#;
        let spec = parse_spec(json).unwrap();
        let by_tag = spec.endpoints_by_tag();
        assert_eq!(by_tag.len(), 2);
        assert!(by_tag.contains_key("users"));
        assert!(by_tag.contains_key("posts"));
    }

    #[test]
    fn generate_example_string_types() {
        let schemas = None;

        let email_schema = Schema {
            schema_type: Some("string".to_string()),
            format: Some("email".to_string()),
            ..Default::default()
        };
        let example = generate_example_from_schema(&email_schema, &schemas).unwrap();
        assert!(example.contains("@"));

        let url_schema = Schema {
            schema_type: Some("string".to_string()),
            format: Some("url".to_string()),
            ..Default::default()
        };
        let example = generate_example_from_schema(&url_schema, &schemas).unwrap();
        assert!(example.contains("https://"));
    }
}

impl Default for Schema {
    fn default() -> Self {
        Self {
            schema_type: None,
            format: None,
            title: None,
            description: None,
            properties: None,
            items: None,
            required: vec![],
            example: None,
            r#ref: None,
            all_of: None,
            one_of: None,
            any_of: None,
            enum_values: None,
        }
    }
}
