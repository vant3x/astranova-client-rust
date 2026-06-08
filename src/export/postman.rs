use crate::persistence::database::{Collection, CollectionFolder, CollectionRequest};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct PostmanCollection {
    info: PostmanInfo,
    item: Vec<PostmanItem>,
    #[serde(rename = "schema")]
    schema: String,
}

#[derive(Serialize, Deserialize)]
struct PostmanInfo {
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct PostmanItem {
    name: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    item: Vec<PostmanItem>,
    #[serde(skip_serializing_if = "Option::is_none")]
    request: Option<PostmanRequest>,
}

#[derive(Serialize, Deserialize)]
struct PostmanRequest {
    method: String,
    #[serde(default)]
    header: Vec<PostmanHeader>,
    #[serde(skip_serializing_if = "Option::is_none")]
    body: Option<PostmanBody>,
    url: PostmanUrl,
}

#[derive(Serialize, Deserialize)]
struct PostmanHeader {
    key: String,
    value: String,
}

#[derive(Serialize, Deserialize)]
struct PostmanBody {
    mode: String,
    raw: String,
}

#[derive(Serialize, Deserialize)]
struct PostmanUrl {
    raw: String,
}

pub fn export_collection(
    collection: &Collection,
    folders: &[CollectionFolder],
    requests: &[CollectionRequest],
) -> Result<String, String> {
    let mut root_items: Vec<PostmanItem> = Vec::new();

    for folder in folders {
        let folder_requests: Vec<PostmanItem> = requests
            .iter()
            .filter(|r| r.folder_id == Some(folder.id))
            .map(request_to_postman_item)
            .collect();

        root_items.push(PostmanItem {
            name: folder.name.clone(),
            item: folder_requests,
            request: None,
        });
    }

    let root_requests: Vec<PostmanItem> = requests
        .iter()
        .filter(|r| r.folder_id.is_none())
        .map(request_to_postman_item)
        .collect();

    root_items.extend(root_requests);

    let collection = PostmanCollection {
        info: PostmanInfo {
            name: collection.name.clone(),
            description: collection.description.clone(),
        },
        item: root_items,
        schema: "https://schema.getpostman.com/json/collection/v2.1.0/collection.json"
            .to_string(),
    };

    serde_json::to_string_pretty(&collection).map_err(|e| format!("Serialization error: {}", e))
}

fn request_to_postman_item(req: &CollectionRequest) -> PostmanItem {
    let headers: Vec<PostmanHeader> = req
        .headers
        .iter()
        .map(|(k, v)| PostmanHeader {
            key: k.clone(),
            value: v.clone(),
        })
        .collect();

    let body = req.body.as_ref().map(|b| PostmanBody {
        mode: "raw".to_string(),
        raw: b.clone(),
    });

    PostmanItem {
        name: req.name.clone(),
        item: Vec::new(),
        request: Some(PostmanRequest {
            method: req.method.clone(),
            header: headers,
            body,
            url: PostmanUrl {
                raw: req.url.clone(),
            },
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_collection(name: &str) -> Collection {
        Collection {
            id: 1,
            name: name.to_string(),
            description: None,
        }
    }

    #[test]
    fn export_empty_collection() {
        let col = make_collection("My API");
        let json = export_collection(&col, &[], &[]).unwrap();
        assert!(json.contains("My API"));
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(parsed["item"].as_array().unwrap().is_empty());
    }

    #[test]
    fn export_collection_with_requests() {
        let col = make_collection("API");
        let requests = vec![CollectionRequest {
            id: 1,
            collection_id: 1,
            folder_id: None,
            name: "Get Users".to_string(),
            method: "GET".to_string(),
            url: "https://api.example.com/users".to_string(),
            headers: vec![],
            body: None,
            body_type: "text".to_string(),
            auth_type: "none".to_string(),
            auth_data: None,
            params: vec![],
            config_json: None,
            sort_order: 0,
        }];

        let json = export_collection(&col, &[], &requests).unwrap();
        assert!(json.contains("Get Users"));
        assert!(json.contains("GET"));
        assert!(json.contains("https://api.example.com/users"));
    }

    #[test]
    fn export_collection_with_folders() {
        let col = make_collection("API");
        let folders = vec![CollectionFolder {
            id: 1,
            collection_id: 1,
            name: "Auth".to_string(),
            parent_folder_id: None,
        }];
        let requests = vec![CollectionRequest {
            id: 1,
            collection_id: 1,
            folder_id: Some(1),
            name: "Login".to_string(),
            method: "POST".to_string(),
            url: "https://api.example.com/login".to_string(),
            headers: vec![("Content-Type".to_string(), "application/json".to_string())],
            body: Some(r#"{"user":"admin"}"#.to_string()),
            body_type: "text".to_string(),
            auth_type: "none".to_string(),
            auth_data: None,
            params: vec![],
            config_json: None,
            sort_order: 0,
        }];

        let json = export_collection(&col, &folders, &requests).unwrap();
        assert!(json.contains("Auth"));
        assert!(json.contains("Login"));
        assert!(json.contains("POST"));
    }

    #[test]
    fn export_preserves_headers() {
        let col = make_collection("API");
        let requests = vec![CollectionRequest {
            id: 1,
            collection_id: 1,
            folder_id: None,
            name: "Auth Request".to_string(),
            method: "GET".to_string(),
            url: "https://api.example.com/data".to_string(),
            headers: vec![
                ("Authorization".to_string(), "Bearer token".to_string()),
                ("Accept".to_string(), "application/json".to_string()),
            ],
            body: None,
            body_type: "text".to_string(),
            auth_type: "none".to_string(),
            auth_data: None,
            params: vec![],
            config_json: None,
            sort_order: 0,
        }];

        let json = export_collection(&col, &[], &requests).unwrap();
        assert!(json.contains("Authorization"));
        assert!(json.contains("Bearer token"));
    }

    #[test]
    fn export_preserves_body() {
        let col = make_collection("API");
        let requests = vec![CollectionRequest {
            id: 1,
            collection_id: 1,
            folder_id: None,
            name: "Create User".to_string(),
            method: "POST".to_string(),
            url: "https://api.example.com/users".to_string(),
            headers: vec![],
            body: Some(r#"{"name":"John"}"#.to_string()),
            body_type: "text".to_string(),
            auth_type: "none".to_string(),
            auth_data: None,
            params: vec![],
            config_json: None,
            sort_order: 0,
        }];

        let json = export_collection(&col, &[], &requests).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        let body = &parsed["item"][0]["request"]["body"]["raw"];
        assert!(body.as_str().unwrap().contains("name"));
        assert!(body.as_str().unwrap().contains("John"));
    }

    #[test]
    fn export_valid_json() {
        let col = make_collection("API");
        let requests = vec![CollectionRequest {
            id: 1,
            collection_id: 1,
            folder_id: None,
            name: "Test".to_string(),
            method: "GET".to_string(),
            url: "https://example.com".to_string(),
            headers: vec![],
            body: None,
            body_type: "text".to_string(),
            auth_type: "none".to_string(),
            auth_data: None,
            params: vec![],
            config_json: None,
            sort_order: 0,
        }];

        let json = export_collection(&col, &[], &requests).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(parsed.is_object());
        assert!(parsed["info"]["name"] == "API");
    }
}
