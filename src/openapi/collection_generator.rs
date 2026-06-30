use super::models::{ParsedEndpoint, ParsedSpec};
use crate::persistence::database::{Collection, CollectionFolder, CollectionRequest};

#[derive(Debug)]
pub struct GeneratedCollection {
    pub collection: Collection,
    pub folders: Vec<CollectionFolder>,
    pub requests: Vec<GeneratedRequest>,
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct GeneratedRequest {
    pub name: String,
    pub method: String,
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: Option<String>,
    pub params: Vec<(String, String)>,
    pub folder_name: Option<String>,
}

pub fn generate_collection(spec: &ParsedSpec, collection_id: i32) -> GeneratedCollection {
    let collection = Collection {
        id: collection_id,
        name: format!("{} (OpenAPI)", spec.title),
        description: spec.description.clone(),
    };

    let by_tag = spec.endpoints_by_tag();
    let mut folders = Vec::new();
    let mut requests = Vec::new();
    let mut folder_id_counter = 1;

    let mut sorted_tags: Vec<&String> = by_tag.keys().collect();
    sorted_tags.sort();

    for tag_name in sorted_tags {
        let endpoints = &by_tag[tag_name];
        let needs_folder = by_tag.len() > 1 || endpoints.len() > 3;

        let folder_id = if needs_folder {
            let folder = CollectionFolder {
                id: folder_id_counter,
                collection_id,
                name: tag_name.clone(),
                parent_folder_id: None,
            };
            folders.push(folder);
            let fid = folder_id_counter;
            folder_id_counter += 1;
            Some(fid)
        } else {
            None
        };

        for endpoint in endpoints {
            let generated = endpoint_to_request(endpoint, collection_id, folder_id, &spec.base_url);
            requests.push(generated);
        }
    }

    GeneratedCollection {
        collection,
        folders,
        requests,
    }
}

fn endpoint_to_request(
    endpoint: &ParsedEndpoint,
    _collection_id: i32,
    _folder_id: Option<i32>,
    base_url: &Option<String>,
) -> GeneratedRequest {
    let name = generate_request_name(endpoint);
    let url = build_url(endpoint, base_url);
    let headers = generate_headers(endpoint);
    let body = endpoint.request_body_example.clone();
    let params = generate_params(endpoint);

    GeneratedRequest {
        name,
        method: endpoint.method.clone(),
        url,
        headers,
        body,
        params,
        folder_name: None,
    }
}

fn generate_request_name(endpoint: &ParsedEndpoint) -> String {
    if let Some(ref op_id) = endpoint.operation_id {
        return op_id.clone();
    }

    if let Some(ref summary) = endpoint.summary {
        return summary.clone();
    }

    let method_display = match endpoint.method.as_str() {
        "GET" => "Get",
        "POST" => "Create",
        "PUT" => "Update",
        "PATCH" => "Patch",
        "DELETE" => "Delete",
        _ => "Request",
    };

    let path_display: String = endpoint
        .path
        .split('/')
        .filter(|s| !s.is_empty() && !s.starts_with('{'))
        .map(|s| {
            let mut chars = s.chars();
            if let Some(first) = chars.next() {
                let upper: String = first.to_uppercase().collect();
                format!("{}{}", upper, chars.as_str())
            } else {
                s.to_string()
            }
        })
        .collect();

    format!("{}{}", method_display, path_display)
}

fn build_url(endpoint: &ParsedEndpoint, base_url: &Option<String>) -> String {
    let base = base_url.as_deref().unwrap_or("http://localhost");
    let base = base.trim_end_matches('/');

    let path = endpoint
        .path
        .split('/')
        .map(|segment| {
            if segment.starts_with('{') && segment.ends_with('}') {
                let param_name = &segment[1..segment.len() - 1];
                format!("{{{}}}", param_name)
            } else {
                segment.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("/");

    if path.starts_with('/') {
        format!("{}{}", base, path)
    } else {
        format!("{}/{}", base, path)
    }
}

fn generate_headers(endpoint: &ParsedEndpoint) -> Vec<(String, String)> {
    let mut headers = vec![("Accept".to_string(), "application/json".to_string())];

    if (endpoint.method == "POST" || endpoint.method == "PUT" || endpoint.method == "PATCH")
        && endpoint.request_body_example.is_some()
    {
        headers.push(("Content-Type".to_string(), "application/json".to_string()));
    }

    headers
}

fn generate_params(endpoint: &ParsedEndpoint) -> Vec<(String, String)> {
    endpoint
        .parameters
        .iter()
        .filter(|p| p.location == "query")
        .filter_map(|p| p.example.as_ref().map(|ex| (p.name.clone(), ex.clone())))
        .collect()
}

#[allow(dead_code)]
pub fn to_collection_requests(
    generated: &GeneratedCollection,
) -> Vec<(CollectionRequest, Option<i32>)> {
    generated
        .requests
        .iter()
        .enumerate()
        .map(|(i, req)| {
            let folder_id = generated.folders.first().map(|f| f.id);

            let collection_req = CollectionRequest {
                id: (i + 1) as i32,
                collection_id: generated.collection.id,
                folder_id,
                name: req.name.clone(),
                method: req.method.clone(),
                url: req.url.clone(),
                headers: req.headers.clone(),
                body: req.body.clone(),
                body_type: "text".to_string(),
                auth_type: "none".to_string(),
                auth_data: None,
                params: req.params.clone(),
                config_json: None,
                sort_order: i as i32,
            };
            (collection_req, folder_id)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_spec(title: &str) -> ParsedSpec {
        ParsedSpec {
            title: title.to_string(),
            description: None,
            version: Some("1.0".to_string()),
            base_url: Some("https://api.example.com".to_string()),
            endpoints: vec![],
            tags: vec![],
        }
    }

    #[test]
    fn generate_request_name_from_operation_id() {
        let endpoint = ParsedEndpoint {
            path: "/users".to_string(),
            method: "GET".to_string(),
            operation_id: Some("listUsers".to_string()),
            summary: Some("List all users".to_string()),
            description: None,
            tags: vec![],
            parameters: vec![],
            request_body_example: None,
            response_example: None,
            deprecated: false,
        };
        assert_eq!(generate_request_name(&endpoint), "listUsers");
    }

    #[test]
    fn generate_request_name_from_summary() {
        let endpoint = ParsedEndpoint {
            path: "/users".to_string(),
            method: "GET".to_string(),
            operation_id: None,
            summary: Some("List all users".to_string()),
            description: None,
            tags: vec![],
            parameters: vec![],
            request_body_example: None,
            response_example: None,
            deprecated: false,
        };
        assert_eq!(generate_request_name(&endpoint), "List all users");
    }

    #[test]
    fn generate_request_name_from_path() {
        let endpoint = ParsedEndpoint {
            path: "/users".to_string(),
            method: "GET".to_string(),
            operation_id: None,
            summary: None,
            description: None,
            tags: vec![],
            parameters: vec![],
            request_body_example: None,
            response_example: None,
            deprecated: false,
        };
        assert_eq!(generate_request_name(&endpoint), "GetUsers");
    }

    #[test]
    fn build_url_with_base() {
        let endpoint = ParsedEndpoint {
            path: "/users/{id}".to_string(),
            method: "GET".to_string(),
            operation_id: None,
            summary: None,
            description: None,
            tags: vec![],
            parameters: vec![],
            request_body_example: None,
            response_example: None,
            deprecated: false,
        };
        let url = build_url(&endpoint, &Some("https://api.example.com/v1".to_string()));
        assert_eq!(url, "https://api.example.com/v1/users/{id}");
    }

    #[test]
    fn build_url_without_base() {
        let endpoint = ParsedEndpoint {
            path: "/users".to_string(),
            method: "GET".to_string(),
            operation_id: None,
            summary: None,
            description: None,
            tags: vec![],
            parameters: vec![],
            request_body_example: None,
            response_example: None,
            deprecated: false,
        };
        let url = build_url(&endpoint, &None);
        assert_eq!(url, "http://localhost/users");
    }

    #[test]
    fn generate_headers_for_post() {
        let endpoint = ParsedEndpoint {
            path: "/users".to_string(),
            method: "POST".to_string(),
            operation_id: None,
            summary: None,
            description: None,
            tags: vec![],
            parameters: vec![],
            request_body_example: Some("{}".to_string()),
            response_example: None,
            deprecated: false,
        };
        let headers = generate_headers(&endpoint);
        assert!(headers
            .iter()
            .any(|(k, v)| k == "Content-Type" && v == "application/json"));
    }

    #[test]
    fn generate_params_from_query_parameters() {
        let endpoint = ParsedEndpoint {
            path: "/users".to_string(),
            method: "GET".to_string(),
            operation_id: None,
            summary: None,
            description: None,
            tags: vec![],
            parameters: vec![
                crate::openapi::models::ParsedParameter {
                    name: "limit".to_string(),
                    location: "query".to_string(),
                    required: false,
                    description: None,
                    example: Some("10".to_string()),
                },
                crate::openapi::models::ParsedParameter {
                    name: "id".to_string(),
                    location: "path".to_string(),
                    required: true,
                    description: None,
                    example: Some("123".to_string()),
                },
            ],
            request_body_example: None,
            response_example: None,
            deprecated: false,
        };
        let params = generate_params(&endpoint);
        assert_eq!(params.len(), 1);
        assert_eq!(params[0].0, "limit");
        assert_eq!(params[0].1, "10");
    }
}
