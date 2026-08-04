use crate::error::AppError;
use crate::openapi::models::{
    Info, MediaType, OpenApiSpec, Operation, Parameter, PathItem, RequestBody, Response, Schema,
    Server, Tag,
};
use crate::persistence::database::{
    Collection, CollectionBodyType, CollectionFolder, CollectionRequest,
};
use std::collections::HashMap;
use std::fmt::Write;

pub fn export_collection_to_openapi(
    collection: &Collection,
    folders: &[CollectionFolder],
    requests: &[CollectionRequest],
) -> Result<String, AppError> {
    let spec = build_openapi_spec(collection, folders, requests);
    serde_json::to_string_pretty(&spec).map_err(|e| AppError::Serialization(e.to_string()))
}

#[allow(dead_code)]
pub fn export_collection_to_openapi_yaml(
    collection: &Collection,
    folders: &[CollectionFolder],
    requests: &[CollectionRequest],
) -> Result<String, AppError> {
    let spec = build_openapi_spec(collection, folders, requests);
    serde_yaml::to_string(&spec).map_err(|e| AppError::Serialization(e.to_string()))
}

fn build_openapi_spec(
    collection: &Collection,
    folders: &[CollectionFolder],
    requests: &[CollectionRequest],
) -> OpenApiSpec {
    let base_url = extract_base_url(requests);

    let tags: Vec<Tag> = folders
        .iter()
        .map(|f| Tag {
            name: f.name.clone(),
            description: None,
        })
        .collect();

    let mut paths: HashMap<String, PathItem> = HashMap::new();

    for req in requests {
        let (path, _base) = extract_path(&req.url, &base_url);
        let tag = folders
            .iter()
            .find(|f| Some(f.id) == req.folder_id)
            .map_or_else(|| "default".to_string(), |f| f.name.clone());

        let operation = request_to_operation(req, &tag);
        let path_item = paths.entry(path).or_insert_with(|| PathItem {
            get: None,
            post: None,
            put: None,
            patch: None,
            delete: None,
            head: None,
            options: None,
            parameters: vec![],
        });

        match req.method.to_uppercase().as_str() {
            "GET" => path_item.get = Some(operation),
            "POST" => path_item.post = Some(operation),
            "PUT" => path_item.put = Some(operation),
            "PATCH" => path_item.patch = Some(operation),
            "DELETE" => path_item.delete = Some(operation),
            "HEAD" => path_item.head = Some(operation),
            "OPTIONS" => path_item.options = Some(operation),
            _ => {}
        }
    }

    let mut sorted_paths: Vec<_> = paths.into_iter().collect();
    sorted_paths.sort_by(|a, b| a.0.cmp(&b.0));
    let paths: HashMap<String, PathItem> = sorted_paths.into_iter().collect();

    let server_url = base_url.unwrap_or_else(|| "http://localhost".to_string());

    OpenApiSpec {
        openapi: Some("3.0.3".to_string()),
        swagger: None,
        info: Info {
            title: collection.name.clone(),
            description: collection.description.clone(),
            version: Some("1.0.0".to_string()),
        },
        servers: vec![Server {
            url: server_url,
            description: None,
        }],
        paths,
        components: None,
        definitions: None,
        tags,
    }
}

fn extract_base_url(requests: &[CollectionRequest]) -> Option<String> {
    let first_url = requests.first()?.url.as_str();
    let parsed = url::Url::parse(first_url).ok()?;
    let scheme = parsed.scheme();
    let host = parsed.host_str()?;
    let port = parsed.port();

    let mut base = format!("{scheme}://{host}");
    if let Some(p) = port {
        let _ = write!(base, ":{p}");
    }
    Some(base)
}

fn extract_path(url: &str, base_url: &Option<String>) -> (String, String) {
    if let Ok(parsed) = url::Url::parse(url) {
        let path = parsed.path().to_string();
        if let Some(base) = base_url {
            if let Ok(base_parsed) = url::Url::parse(base) {
                let base_path = base_parsed.path();
                if path.starts_with(base_path) && base_path != "/" {
                    let relative = &path[base_path.len()..];
                    let normalized = if relative.is_empty() {
                        "/".to_string()
                    } else {
                        format!("/{}", relative.trim_start_matches('/'))
                    };
                    return (normalize_path(&normalized), "GET".to_string());
                }
            }
        }
        let normalized = if path.is_empty() {
            "/".to_string()
        } else {
            path
        };
        return (normalize_path(&normalized), "GET".to_string());
    }
    (normalize_path(url), "GET".to_string())
}

fn normalize_path(path: &str) -> String {
    let parts: Vec<&str> = path.trim_matches('/').split('/').collect();
    let normalized: Vec<String> = parts
        .iter()
        .map(|part| {
            if part.parse::<i64>().is_ok() || part.parse::<f64>().is_ok() {
                "{id}".to_string()
            } else {
                part.to_string()
            }
        })
        .collect();
    format!("/{}", normalized.join("/"))
}

fn request_to_operation(req: &CollectionRequest, tag: &str) -> Operation {
    let mut parameters = Vec::new();

    for (key, _value) in &req.headers {
        if !key.eq_ignore_ascii_case("content-type")
            && !key.eq_ignore_ascii_case("accept")
            && !key.eq_ignore_ascii_case("cookie")
        {
            parameters.push(Parameter {
                name: key.clone(),
                location: "header".to_string(),
                description: None,
                required: true,
                schema: Some(Schema {
                    schema_type: Some("string".to_string()),
                    ..Default::default()
                }),
                param_type: Some("string".to_string()),
                example: None,
            });
        }
    }

    for (key, _value) in &req.params {
        parameters.push(Parameter {
            name: key.clone(),
            location: "query".to_string(),
            description: None,
            required: false,
            schema: Some(Schema {
                schema_type: Some("string".to_string()),
                ..Default::default()
            }),
            param_type: Some("string".to_string()),
            example: None,
        });
    }

    let request_body = build_request_body(req);

    Operation {
        summary: Some(req.name.clone()),
        description: None,
        operation_id: Some(slugify(&req.name)),
        tags: vec![tag.to_string()],
        parameters,
        request_body,
        responses: build_responses(),
        deprecated: false,
        security: vec![],
    }
}

fn build_request_body(req: &CollectionRequest) -> Option<RequestBody> {
    let body = req.body.as_deref()?;

    if body.is_empty() {
        return None;
    }

    let content_type = map_body_type(&req.body_type);
    let mut content = HashMap::new();

    let example = if req.body_type == CollectionBodyType::Json {
        serde_json::from_str(body).ok()
    } else {
        Some(serde_json::Value::String(body.to_string()))
    };

    content.insert(
        content_type.to_string(),
        MediaType {
            schema: None,
            example,
        },
    );

    Some(RequestBody {
        description: None,
        content,
        required: true,
    })
}

fn build_responses() -> HashMap<String, Response> {
    let mut responses = HashMap::new();
    responses.insert(
        "200".to_string(),
        Response {
            description: Some("Successful response".to_string()),
            content: HashMap::new(),
        },
    );
    responses
}

fn map_body_type(body_type: &CollectionBodyType) -> &'static str {
    match body_type {
        CollectionBodyType::Json => "application/json",
        CollectionBodyType::Xml => "application/xml",
        CollectionBodyType::FormUrlencoded => "application/x-www-form-urlencoded",
        CollectionBodyType::Multipart => "multipart/form-data",
        CollectionBodyType::Text => "text/plain",
        CollectionBodyType::Html => "text/html",
        CollectionBodyType::Graphql => "application/json",
        CollectionBodyType::Binary | CollectionBodyType::None => "application/octet-stream",
    }
}

fn slugify(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_alphanumeric() {
                c.to_lowercase().next().unwrap_or(c)
            } else {
                '_'
            }
        })
        .collect::<String>()
        .split('_')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("_")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_collection() -> Collection {
        Collection {
            id: 1,
            name: "Test API".to_string(),
            description: Some("A test API".to_string()),
            sort_order: 0,
            variables: vec![],
        }
    }

    fn test_request(name: &str, method: &str, url: &str) -> CollectionRequest {
        CollectionRequest {
            id: 1,
            collection_id: 1,
            folder_id: None,
            name: name.to_string(),
            method: method.to_string(),
            url: url.to_string(),
            headers: vec![],
            body: None,
            body_type: CollectionBodyType::None,
            auth_type: Default::default(),
            auth_data: None,
            params: vec![],
            config_json: None,
            scripts: None,
            sort_order: 0,
        }
    }

    #[test]
    fn test_extract_base_url() {
        let requests = vec![test_request(
            "test",
            "GET",
            "https://api.example.com/v1/users",
        )];
        let base = extract_base_url(&requests);
        assert_eq!(base, Some("https://api.example.com".to_string()));
    }

    #[test]
    fn test_extract_path_with_base() {
        let base = Some("https://api.example.com/v1".to_string());
        let (path, _) = extract_path("https://api.example.com/v1/users/123", &base);
        assert_eq!(path, "/users/{id}");
    }

    #[test]
    fn test_normalize_path_numeric_segments() {
        assert_eq!(normalize_path("/users/123"), "/users/{id}");
        assert_eq!(
            normalize_path("/users/123/posts/456"),
            "/users/{id}/posts/{id}"
        );
        assert_eq!(normalize_path("/users"), "/users");
    }

    #[test]
    fn test_slugify() {
        assert_eq!(slugify("Get Users"), "get_users");
        assert_eq!(slugify("Create New Post"), "create_new_post");
        assert_eq!(slugify("user-login"), "user_login");
    }

    #[test]
    fn test_export_produces_valid_json() {
        let col = test_collection();
        let result = export_collection_to_openapi(&col, &[], &[]);
        assert!(result.is_ok());
        let json = result.unwrap();
        assert!(json.contains("3.0.3"));
        assert!(json.contains("Test API"));
    }

    #[test]
    fn test_export_with_requests() {
        let col = test_collection();
        let requests = vec![
            test_request("Get Users", "GET", "https://api.example.com/v1/users"),
            test_request("Create User", "POST", "https://api.example.com/v1/users"),
        ];
        let result = export_collection_to_openapi(&col, &[], &requests);
        assert!(result.is_ok());
        let json = result.unwrap();
        assert!(json.contains("/users"));
        assert!(json.contains("Get Users"));
    }

    #[test]
    fn test_export_with_folders_as_tags() {
        let col = test_collection();
        let folders = vec![CollectionFolder {
            id: 1,
            collection_id: 1,
            name: "Users".to_string(),
            parent_folder_id: None,
            sort_order: 0,
        }];
        let requests = vec![CollectionRequest {
            folder_id: Some(1),
            ..test_request("Get User", "GET", "https://api.example.com/users/1")
        }];
        let result = export_collection_to_openapi(&col, &folders, &requests);
        assert!(result.is_ok());
        let json = result.unwrap();
        assert!(json.contains("Users"));
    }
}
