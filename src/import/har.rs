use crate::error::AppError;
use crate::export::har::HarLog;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct HarRoot {
    log: HarLog,
}

pub fn parse_har_collection(
    json: &str,
) -> Result<crate::import::postman::ImportedCollection, AppError> {
    let root: HarRoot = serde_json::from_str(json)
        .map_err(|e| AppError::Parse(format!("Invalid HAR JSON: {e}")))?;

    use std::collections::HashMap;
    let mut folders_map: HashMap<String, Vec<crate::import::postman::ImportedRequest>> =
        HashMap::new();
    let mut root_requests = Vec::new();

    for entry in &root.log.entries {
        let req = &entry.request;
        let url = req.url.clone();

        let host = url::Url::parse(&url)
            .ok()
            .and_then(|u| u.host_str().map(std::string::ToString::to_string));

        let path = url::Url::parse(&url)
            .ok()
            .map_or_else(|| url.clone(), |u| u.path().to_string());
        let name = format!("{} {}", req.method, path);

        let headers: Vec<(String, String)> = req
            .headers
            .iter()
            .map(|h| (h.name.clone(), h.value.clone()))
            .collect();

        let body = req.post_data.as_ref().and_then(|pd| pd.text.clone());

        let params: Vec<(String, String)> = req
            .query_string
            .iter()
            .map(|qp| (qp.name.clone(), qp.value.clone()))
            .collect();

        let imported_req = crate::import::postman::ImportedRequest {
            name,
            method: req.method.clone(),
            url,
            headers,
            body,
            params,
            scripts: None,
        };

        if let Some(h) = host {
            folders_map.entry(h).or_default().push(imported_req);
        } else {
            root_requests.push(imported_req);
        }
    }

    let mut folders = Vec::new();
    for (host_name, reqs) in folders_map {
        folders.push(crate::import::postman::ImportedFolder {
            name: host_name,
            requests: reqs,
        });
    }

    let col_name = if root.log.creator.name.is_empty() {
        "Imported HAR Collection".to_string()
    } else {
        format!("HAR - {}", root.log.creator.name)
    };

    Ok(crate::import::postman::ImportedCollection {
        name: col_name,
        description: Some(format!("Imported from HAR v{}", root.log.version)),
        folders,
        requests: root_requests,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_har() {
        let json_str = r#"{
            "log": {
                "version": "1.2",
                "creator": {
                    "name": "WebBrowser",
                    "version": "1.0"
                },
                "entries": [
                    {
                        "startedDateTime": "2026-07-19T06:51:19.000Z",
                        "time": 100,
                        "request": {
                            "method": "GET",
                            "url": "https://api.example.com/v1/users?limit=10",
                            "httpVersion": "HTTP/1.1",
                            "headers": [
                                { "name": "Accept", "value": "application/json" }
                            ],
                            "queryString": [
                                { "name": "limit", "value": "10" }
                            ],
                            "cookies": []
                        },
                        "response": {
                            "status": 200,
                            "statusText": "OK",
                            "httpVersion": "HTTP/1.1",
                            "headers": [],
                            "cookies": [],
                            "content": {
                                "size": 0,
                                "mimeType": "application/json"
                            },
                            "redirectUrl": ""
                        },
                        "timings": {
                            "send": 0,
                            "wait": 100,
                            "receive": 0
                        }
                    }
                ]
            }
        }"#;

        let col = parse_har_collection(json_str).unwrap();
        assert_eq!(col.name, "HAR - WebBrowser");
        assert_eq!(col.folders.len(), 1);
        assert_eq!(col.folders[0].name, "api.example.com");
        assert_eq!(col.folders[0].requests.len(), 1);

        let req = &col.folders[0].requests[0];
        assert_eq!(req.name, "GET /v1/users");
        assert_eq!(req.method, "GET");
        assert_eq!(req.url, "https://api.example.com/v1/users?limit=10");
        assert_eq!(req.headers.len(), 1);
        assert_eq!(
            req.headers[0],
            ("Accept".to_string(), "application/json".to_string())
        );
        assert_eq!(req.params.len(), 1);
        assert_eq!(req.params[0], ("limit".to_string(), "10".to_string()));
    }
}
