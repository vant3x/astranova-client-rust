use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct PostmanCollection {
    pub info: PostmanInfo,
    #[serde(default)]
    pub item: Vec<PostmanItem>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PostmanInfo {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PostmanItem {
    pub name: String,
    #[serde(default)]
    pub item: Vec<PostmanItem>,
    #[serde(default)]
    pub request: Option<PostmanRequest>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PostmanRequest {
    #[serde(default)]
    pub method: String,
    #[serde(default)]
    pub header: Vec<PostmanHeader>,
    #[serde(default)]
    pub body: Option<PostmanBody>,
    #[serde(default)]
    pub url: Option<PostmanUrl>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PostmanHeader {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PostmanBody {
    pub mode: String,
    #[serde(default)]
    pub raw: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PostmanUrl {
    #[serde(default)]
    pub raw: Option<String>,
    #[serde(default)]
    pub protocol: Option<String>,
    #[serde(default)]
    pub host: Option<Vec<String>>,
    #[serde(default)]
    pub path: Option<Vec<String>>,
    #[serde(default)]
    pub query: Option<Vec<PostmanQueryParam>>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PostmanQueryParam {
    pub key: String,
    pub value: String,
}

#[derive(Debug)]
pub struct ImportedCollection {
    pub name: String,
    pub description: Option<String>,
    pub folders: Vec<ImportedFolder>,
    pub requests: Vec<ImportedRequest>,
}

#[derive(Debug)]
pub struct ImportedFolder {
    pub name: String,
    pub requests: Vec<ImportedRequest>,
}

#[derive(Debug)]
pub struct ImportedRequest {
    pub name: String,
    pub method: String,
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: Option<String>,
    pub params: Vec<(String, String)>,
}

pub fn parse_postman_collection(json: &str) -> Result<ImportedCollection, String> {
    let collection: PostmanCollection =
        serde_json::from_str(json).map_err(|e| format!("Invalid Postman collection: {}", e))?;

    let mut imported_folders = Vec::new();
    let mut imported_requests = Vec::new();

    for item in &collection.item {
        if item.request.is_some() {
            if let Some(req) = parse_postman_request(item) {
                imported_requests.push(req);
            }
        } else if !item.item.is_empty() {
            let mut folder_requests = Vec::new();
            for sub_item in &item.item {
                if let Some(req) = parse_postman_request(sub_item) {
                    folder_requests.push(req);
                }
            }
            imported_folders.push(ImportedFolder {
                name: item.name.clone(),
                requests: folder_requests,
            });
        }
    }

    Ok(ImportedCollection {
        name: collection.info.name,
        description: collection.info.description,
        folders: imported_folders,
        requests: imported_requests,
    })
}

fn parse_postman_request(item: &PostmanItem) -> Option<ImportedRequest> {
    let request = item.request.as_ref()?;

    let url = extract_url(request.url.as_ref()?);
    let headers = request
        .header
        .iter()
        .map(|h| (h.key.clone(), h.value.clone()))
        .collect();
    let body = request.body.as_ref().and_then(|b| b.raw.clone());
    let params = extract_params(request.url.as_ref()?);

    Some(ImportedRequest {
        name: item.name.clone(),
        method: if request.method.is_empty() {
            "GET".to_string()
        } else {
            request.method.clone()
        },
        url,
        headers,
        body,
        params,
    })
}

fn extract_url(url: &PostmanUrl) -> String {
    if let Some(raw) = &url.raw {
        return raw.clone();
    }

    let mut parts = Vec::new();

    if let Some(protocol) = &url.protocol {
        parts.push(format!("{}://", protocol));
    }

    if let Some(host) = &url.host {
        parts.push(host.join("."));
    }

    if let Some(path) = &url.path {
        parts.push(format!("/{}", path.join("/")));
    }

    parts.join("")
}

fn extract_params(url: &PostmanUrl) -> Vec<(String, String)> {
    url.query
        .as_ref()
        .map(|q| q.iter().map(|p| (p.key.clone(), p.value.clone())).collect())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_collection() {
        let json = r#"{
            "info": { "name": "My API" },
            "item": [
                {
                    "name": "Get Users",
                    "request": {
                        "method": "GET",
                        "header": [],
                        "url": { "raw": "https://api.example.com/users" }
                    }
                }
            ]
        }"#;

        let collection = parse_postman_collection(json).unwrap();
        assert_eq!(collection.name, "My API");
        assert_eq!(collection.requests.len(), 1);
        assert_eq!(collection.requests[0].name, "Get Users");
        assert_eq!(collection.requests[0].method, "GET");
        assert_eq!(collection.requests[0].url, "https://api.example.com/users");
    }

    #[test]
    fn parse_collection_with_folders() {
        let json = r#"{
            "info": { "name": "API" },
            "item": [
                {
                    "name": "Auth",
                    "item": [
                        {
                            "name": "Login",
                            "request": {
                                "method": "POST",
                                "header": [{ "key": "Content-Type", "value": "application/json" }],
                                "body": { "mode": "raw", "raw": "{\"user\":\"admin\"}" },
                                "url": { "raw": "https://api.example.com/login" }
                            }
                        }
                    ]
                }
            ]
        }"#;

        let collection = parse_postman_collection(json).unwrap();
        assert_eq!(collection.folders.len(), 1);
        assert_eq!(collection.folders[0].name, "Auth");
        assert_eq!(collection.folders[0].requests.len(), 1);
        assert_eq!(collection.folders[0].requests[0].name, "Login");
        assert_eq!(collection.folders[0].requests[0].method, "POST");
    }

    #[test]
    fn parse_request_with_params() {
        let json = r#"{
            "info": { "name": "API" },
            "item": [
                {
                    "name": "Search",
                    "request": {
                        "method": "GET",
                        "header": [],
                        "url": {
                            "raw": "https://api.example.com/search?q=test&page=1",
                            "query": [
                                { "key": "q", "value": "test" },
                                { "key": "page", "value": "1" }
                            ]
                        }
                    }
                }
            ]
        }"#;

        let collection = parse_postman_collection(json).unwrap();
        assert_eq!(collection.requests[0].params.len(), 2);
        assert_eq!(collection.requests[0].params[0].0, "q");
        assert_eq!(collection.requests[0].params[0].1, "test");
    }

    #[test]
    fn parse_request_with_headers() {
        let json = r#"{
            "info": { "name": "API" },
            "item": [
                {
                    "name": "Auth Request",
                    "request": {
                        "method": "GET",
                        "header": [
                            { "key": "Authorization", "value": "Bearer token123" },
                            { "key": "Accept", "value": "application/json" }
                        ],
                        "url": { "raw": "https://api.example.com/data" }
                    }
                }
            ]
        }"#;

        let collection = parse_postman_collection(json).unwrap();
        assert_eq!(collection.requests[0].headers.len(), 2);
        assert_eq!(collection.requests[0].headers[0].0, "Authorization");
    }

    #[test]
    fn parse_request_with_body() {
        let json = r#"{
            "info": { "name": "API" },
            "item": [
                {
                    "name": "Create User",
                    "request": {
                        "method": "POST",
                        "header": [],
                        "body": { "mode": "raw", "raw": "{\"name\":\"John\"}" },
                        "url": { "raw": "https://api.example.com/users" }
                    }
                }
            ]
        }"#;

        let collection = parse_postman_collection(json).unwrap();
        assert_eq!(
            collection.requests[0].body,
            Some("{\"name\":\"John\"}".to_string())
        );
    }

    #[test]
    fn parse_collection_with_description() {
        let json = r#"{
            "info": { "name": "API", "description": "My API endpoints" },
            "item": []
        }"#;

        let collection = parse_postman_collection(json).unwrap();
        assert_eq!(collection.description, Some("My API endpoints".to_string()));
    }

    #[test]
    fn parse_url_from_components() {
        let json = r#"{
            "info": { "name": "API" },
            "item": [
                {
                    "name": "Test",
                    "request": {
                        "method": "GET",
                        "header": [],
                        "url": {
                            "protocol": "https",
                            "host": ["api", "example", "com"],
                            "path": ["v1", "users"]
                        }
                    }
                }
            ]
        }"#;

        let collection = parse_postman_collection(json).unwrap();
        assert_eq!(
            collection.requests[0].url,
            "https://api.example.com/v1/users"
        );
    }

    #[test]
    fn parse_invalid_json_returns_error() {
        let result = parse_postman_collection("not json");
        assert!(result.is_err());
    }
}
