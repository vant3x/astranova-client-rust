use crate::error::AppError;
use directories::ProjectDirs;
use rusqlite::{params, Connection, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Environment {
    pub id: i32,
    pub name: String,
    pub variables: Vec<(String, String)>,
    pub default_endpoint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestHistoryEntry {
    pub id: i32,
    pub method: String,
    pub url: String,
    pub status: Option<u16>,
    pub duration_ms: Option<u64>,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Collection {
    pub id: i32,
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CollectionFolder {
    pub id: i32,
    pub collection_id: i32,
    pub name: String,
    pub parent_folder_id: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CollectionRequest {
    pub id: i32,
    pub collection_id: i32,
    pub folder_id: Option<i32>,
    pub name: String,
    pub method: String,
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: Option<String>,
    pub body_type: String,
    pub auth_type: String,
    pub auth_data: Option<String>,
    pub params: Vec<(String, String)>,
    pub config_json: Option<String>,
    pub sort_order: i32,
}

impl std::fmt::Display for Collection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

impl std::fmt::Display for CollectionFolder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

impl std::fmt::Display for Environment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

fn get_db_path() -> std::result::Result<PathBuf, AppError> {
    let proj_dirs =
        ProjectDirs::from("com", "astranova", "client").ok_or_else(|| {
            AppError::Database("Failed to determine project directories".to_string())
        })?;
    let data_dir = proj_dirs.data_dir();
    std::fs::create_dir_all(data_dir)
        .map_err(|e| AppError::Io(format!("Failed to create data directory: {}", e)))?;
    Ok(data_dir.join("astranova.db"))
}

pub fn init() -> std::result::Result<Connection, AppError> {
    let db_path = get_db_path()?;
    let conn = Connection::open(db_path)?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS environments (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL UNIQUE,
            variables TEXT NOT NULL
        )",
        [],
    )?;
    conn.execute(
        "ALTER TABLE environments ADD COLUMN default_endpoint TEXT",
        [],
    )
    .ok();
    conn.execute(
        "CREATE TABLE IF NOT EXISTS request_history (
            id INTEGER PRIMARY KEY,
            method TEXT NOT NULL,
            url TEXT NOT NULL,
            status INTEGER,
            duration_ms INTEGER,
            timestamp TEXT NOT NULL
        )",
        [],
    )?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS collections (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            description TEXT
        )",
        [],
    )?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS collection_folders (
            id INTEGER PRIMARY KEY,
            collection_id INTEGER NOT NULL,
            name TEXT NOT NULL,
            parent_folder_id INTEGER,
            FOREIGN KEY (collection_id) REFERENCES collections(id) ON DELETE CASCADE,
            FOREIGN KEY (parent_folder_id) REFERENCES collection_folders(id) ON DELETE CASCADE
        )",
        [],
    )?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS collection_requests (
            id INTEGER PRIMARY KEY,
            collection_id INTEGER NOT NULL,
            folder_id INTEGER,
            name TEXT NOT NULL,
            method TEXT NOT NULL,
            url TEXT NOT NULL,
            headers TEXT NOT NULL DEFAULT '[]',
            body TEXT,
            body_type TEXT NOT NULL DEFAULT 'text',
            auth_type TEXT NOT NULL DEFAULT 'none',
            auth_data TEXT,
            params TEXT NOT NULL DEFAULT '[]',
            config_json TEXT,
            sort_order INTEGER NOT NULL DEFAULT 0,
            FOREIGN KEY (collection_id) REFERENCES collections(id) ON DELETE CASCADE,
            FOREIGN KEY (folder_id) REFERENCES collection_folders(id) ON DELETE CASCADE
        )",
        [],
    )?;
    Ok(conn)
}

pub fn create_environment(conn: &Connection, name: &str) -> Result<Environment> {
    let variables: Vec<(String, String)> = Vec::new();
    let variables_json =
        serde_json::to_value(&variables).map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;
    conn.execute(
        "INSERT INTO environments (name, variables) VALUES (?1, ?2)",
        [name, &variables_json.to_string()],
    )?;
    let id = conn.last_insert_rowid();
    Ok(Environment {
        id: id as i32,
        name: name.to_string(),
        variables,
        default_endpoint: None,
    })
}

pub fn get_environments(conn: &Connection) -> Result<Vec<Environment>> {
    let mut stmt =
        conn.prepare("SELECT id, name, variables, default_endpoint FROM environments")?;
    let env_iter = stmt.query_map([], |row| {
        let variables_json: String = row.get(2)?;
        let variables: Vec<(String, String)> =
            serde_json::from_str(&variables_json).unwrap_or_default();
        Ok(Environment {
            id: row.get(0)?,
            name: row.get(1)?,
            variables,
            default_endpoint: row.get(3)?,
        })
    })?;

    let mut environments = Vec::new();
    for env in env_iter {
        environments.push(env?);
    }
    Ok(environments)
}

pub fn update_environment(conn: &Connection, env: &Environment) -> Result<()> {
    let variables_json =
        serde_json::to_value(&env.variables).map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;
    conn.execute(
        "UPDATE environments SET name = ?1, variables = ?2, default_endpoint = ?3 WHERE id = ?4",
        params![
            &env.name,
            &variables_json.to_string(),
            &env.default_endpoint,
            &env.id.to_string(),
        ],
    )?;
    Ok(())
}

pub fn delete_environment(conn: &Connection, id: i32) -> Result<()> {
    conn.execute("DELETE FROM environments WHERE id = ?1", [&id.to_string()])?;
    Ok(())
}

pub fn save_request_history(
    conn: &Connection,
    method: &str,
    url: &str,
    status: Option<u16>,
    duration_ms: Option<u64>,
) -> Result<()> {
    let timestamp = chrono_now();
    conn.execute(
        "INSERT INTO request_history (method, url, status, duration_ms, timestamp) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![method, url, status.map(|s| s as i64), duration_ms.map(|d| d as i64), timestamp],
    )?;
    Ok(())
}

pub fn get_request_history(conn: &Connection, limit: usize) -> Result<Vec<RequestHistoryEntry>> {
    let mut stmt = conn.prepare(
        "SELECT id, method, url, status, duration_ms, timestamp FROM request_history ORDER BY id DESC LIMIT ?1",
    )?;
    let entries = stmt.query_map([limit as i64], |row| {
        Ok(RequestHistoryEntry {
            id: row.get(0)?,
            method: row.get(1)?,
            url: row.get(2)?,
            status: row.get::<_, Option<i64>>(3)?.map(|s| s as u16),
            duration_ms: row.get::<_, Option<i64>>(4)?.map(|d| d as u64),
            timestamp: row.get(5)?,
        })
    })?;

    let mut result = Vec::new();
    for entry in entries {
        result.push(entry?);
    }
    Ok(result)
}

#[allow(dead_code)]
pub fn delete_request_history(conn: &Connection) -> Result<()> {
    conn.execute("DELETE FROM request_history", [])?;
    Ok(())
}

pub fn create_collection(conn: &Connection, name: &str, description: Option<&str>) -> Result<Collection> {
    conn.execute(
        "INSERT INTO collections (name, description) VALUES (?1, ?2)",
        params![name, description],
    )?;
    let id = conn.last_insert_rowid();
    Ok(Collection {
        id: id as i32,
        name: name.to_string(),
        description: description.map(|s| s.to_string()),
    })
}

pub fn get_collections(conn: &Connection) -> Result<Vec<Collection>> {
    let mut stmt = conn.prepare("SELECT id, name, description FROM collections ORDER BY name")?;
    let rows = stmt.query_map([], |row| {
        Ok(Collection {
            id: row.get(0)?,
            name: row.get(1)?,
            description: row.get(2)?,
        })
    })?;
    rows.collect()
}

#[allow(dead_code)]
pub fn update_collection(conn: &Connection, collection: &Collection) -> Result<()> {
    conn.execute(
        "UPDATE collections SET name = ?1, description = ?2 WHERE id = ?3",
        params![collection.name, collection.description, collection.id],
    )?;
    Ok(())
}

pub fn delete_collection(conn: &Connection, id: i32) -> Result<()> {
    conn.execute("DELETE FROM collections WHERE id = ?1", [id])?;
    Ok(())
}

pub fn create_folder(conn: &Connection, collection_id: i32, name: &str, parent_folder_id: Option<i32>) -> Result<CollectionFolder> {
    conn.execute(
        "INSERT INTO collection_folders (collection_id, name, parent_folder_id) VALUES (?1, ?2, ?3)",
        params![collection_id, name, parent_folder_id],
    )?;
    let id = conn.last_insert_rowid();
    Ok(CollectionFolder {
        id: id as i32,
        collection_id,
        name: name.to_string(),
        parent_folder_id,
    })
}

pub fn get_folders(conn: &Connection, collection_id: i32) -> Result<Vec<CollectionFolder>> {
    let mut stmt = conn.prepare(
        "SELECT id, collection_id, name, parent_folder_id FROM collection_folders WHERE collection_id = ?1 ORDER BY name",
    )?;
    let rows = stmt.query_map([collection_id], |row| {
        Ok(CollectionFolder {
            id: row.get(0)?,
            collection_id: row.get(1)?,
            name: row.get(2)?,
            parent_folder_id: row.get(3)?,
        })
    })?;
    rows.collect()
}

pub fn delete_folder(conn: &Connection, id: i32) -> Result<()> {
    conn.execute("DELETE FROM collection_folders WHERE id = ?1", [id])?;
    Ok(())
}

#[allow(dead_code)]
pub fn rename_folder(conn: &Connection, id: i32, new_name: &str) -> Result<()> {
    conn.execute(
        "UPDATE collection_folders SET name = ?1 WHERE id = ?2",
        params![new_name, id],
    )?;
    Ok(())
}

pub fn save_collection_request(
    conn: &Connection,
    collection_id: i32,
    folder_id: Option<i32>,
    name: &str,
    method: &str,
    url: &str,
    headers: &[(String, String)],
    body: Option<&str>,
    body_type: &str,
    auth_type: &str,
    auth_data: Option<&str>,
    params: &[(String, String)],
    config_json: Option<&str>,
) -> Result<CollectionRequest> {
    let headers_json = serde_json::to_string(headers)
        .map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;
    let params_json = serde_json::to_string(params)
        .map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;
    let max_order: i32 = conn
        .query_row(
            "SELECT COALESCE(MAX(sort_order), 0) FROM collection_requests WHERE collection_id = ?1",
            [collection_id],
            |row| row.get(0),
        )
        .unwrap_or(0);

    conn.execute(
        "INSERT INTO collection_requests (collection_id, folder_id, name, method, url, headers, body, body_type, auth_type, auth_data, params, config_json, sort_order) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
        params![
            collection_id,
            folder_id,
            name,
            method,
            url,
            headers_json,
            body,
            body_type,
            auth_type,
            auth_data,
            params_json,
            config_json,
            max_order + 1,
        ],
    )?;
    let id = conn.last_insert_rowid();
    Ok(CollectionRequest {
        id: id as i32,
        collection_id,
        folder_id,
        name: name.to_string(),
        method: method.to_string(),
        url: url.to_string(),
        headers: headers.to_vec(),
        body: body.map(|s| s.to_string()),
        body_type: body_type.to_string(),
        auth_type: auth_type.to_string(),
        auth_data: auth_data.map(|s| s.to_string()),
        params: params.to_vec(),
        config_json: config_json.map(|s| s.to_string()),
        sort_order: max_order + 1,
    })
}

pub fn get_collection_requests(conn: &Connection, collection_id: i32, folder_id: Option<i32>) -> Result<Vec<CollectionRequest>> {
    let mut stmt = conn.prepare(
        "SELECT id, collection_id, folder_id, name, method, url, headers, body, body_type, auth_type, auth_data, params, config_json, sort_order FROM collection_requests WHERE collection_id = ?1 AND folder_id IS ?2 ORDER BY sort_order",
    )?;
    let rows = stmt.query_map(params![collection_id, folder_id], |row| {
        parse_collection_request(row)
    })?;
    rows.collect()
}

fn parse_collection_request(row: &rusqlite::Row) -> rusqlite::Result<CollectionRequest> {
    let headers_json: String = row.get(6)?;
    let params_json: String = row.get(11)?;
    Ok(CollectionRequest {
        id: row.get(0)?,
        collection_id: row.get(1)?,
        folder_id: row.get(2)?,
        name: row.get(3)?,
        method: row.get(4)?,
        url: row.get(5)?,
        headers: serde_json::from_str(&headers_json).unwrap_or_default(),
        body: row.get(7)?,
        body_type: row.get(8)?,
        auth_type: row.get(9)?,
        auth_data: row.get(10)?,
        params: serde_json::from_str(&params_json).unwrap_or_default(),
        config_json: row.get(12)?,
        sort_order: row.get(13)?,
    })
}

#[allow(dead_code)]
pub fn rename_collection_request(conn: &Connection, id: i32, new_name: &str) -> Result<()> {
    conn.execute(
        "UPDATE collection_requests SET name = ?1 WHERE id = ?2",
        params![new_name, id],
    )?;
    Ok(())
}

#[allow(dead_code)]
pub fn move_collection_request(conn: &Connection, id: i32, new_folder_id: Option<i32>) -> Result<()> {
    conn.execute(
        "UPDATE collection_requests SET folder_id = ?1 WHERE id = ?2",
        params![new_folder_id, id],
    )?;
    Ok(())
}

#[allow(dead_code)]
pub fn delete_collection_request(conn: &Connection, id: i32) -> Result<()> {
    conn.execute("DELETE FROM collection_requests WHERE id = ?1", [id])?;
    Ok(())
}

fn chrono_now() -> String {
    use std::time::SystemTime;
    let duration = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = duration.as_secs();
    format!("{}", secs)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_test_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute(
            "CREATE TABLE IF NOT EXISTS environments (
                id INTEGER PRIMARY KEY,
                name TEXT NOT NULL UNIQUE,
                variables TEXT NOT NULL
            )",
            [],
        )
        .unwrap();
        conn.execute(
            "ALTER TABLE environments ADD COLUMN default_endpoint TEXT",
            [],
        )
        .ok();
        conn.execute(
            "CREATE TABLE IF NOT EXISTS collections (
                id INTEGER PRIMARY KEY,
                name TEXT NOT NULL,
                description TEXT
            )",
            [],
        )
        .unwrap();
        conn.execute(
            "CREATE TABLE IF NOT EXISTS collection_folders (
                id INTEGER PRIMARY KEY,
                collection_id INTEGER NOT NULL,
                name TEXT NOT NULL,
                parent_folder_id INTEGER
            )",
            [],
        )
        .unwrap();
        conn.execute(
            "CREATE TABLE IF NOT EXISTS collection_requests (
                id INTEGER PRIMARY KEY,
                collection_id INTEGER NOT NULL,
                folder_id INTEGER,
                name TEXT NOT NULL,
                method TEXT NOT NULL,
                url TEXT NOT NULL,
                headers TEXT NOT NULL DEFAULT '[]',
                body TEXT,
                body_type TEXT NOT NULL DEFAULT 'text',
                auth_type TEXT NOT NULL DEFAULT 'none',
                auth_data TEXT,
                params TEXT NOT NULL DEFAULT '[]',
                config_json TEXT,
                sort_order INTEGER NOT NULL DEFAULT 0
            )",
            [],
        )
        .unwrap();
        conn
    }

    #[test]
    fn create_and_get_environment() {
        let conn = setup_test_db();
        let env = create_environment(&conn, "test-env").unwrap();
        assert_eq!(env.name, "test-env");
        assert!(env.variables.is_empty());
        assert!(env.default_endpoint.is_none());

        let envs = get_environments(&conn).unwrap();
        assert_eq!(envs.len(), 1);
        assert_eq!(envs[0].name, "test-env");
    }

    #[test]
    fn create_multiple_environments() {
        let conn = setup_test_db();
        create_environment(&conn, "env-1").unwrap();
        create_environment(&conn, "env-2").unwrap();
        create_environment(&conn, "env-3").unwrap();

        let envs = get_environments(&conn).unwrap();
        assert_eq!(envs.len(), 3);
    }

    #[test]
    fn update_environment_name() {
        let conn = setup_test_db();
        let mut env = create_environment(&conn, "original").unwrap();
        env.name = "updated".to_string();
        update_environment(&conn, &env).unwrap();

        let envs = get_environments(&conn).unwrap();
        assert_eq!(envs[0].name, "updated");
    }

    #[test]
    fn update_environment_variables() {
        let conn = setup_test_db();
        let mut env = create_environment(&conn, "with-vars").unwrap();
        env.variables = vec![
            ("API_URL".to_string(), "https://api.example.com".to_string()),
            ("TOKEN".to_string(), "abc123".to_string()),
        ];
        update_environment(&conn, &env).unwrap();

        let envs = get_environments(&conn).unwrap();
        assert_eq!(envs[0].variables.len(), 2);
        assert_eq!(envs[0].variables[0].0, "API_URL");
        assert_eq!(envs[0].variables[0].1, "https://api.example.com");
    }

    #[test]
    fn update_environment_endpoint() {
        let conn = setup_test_db();
        let mut env = create_environment(&conn, "with-endpoint").unwrap();
        env.default_endpoint = Some("https://api.example.com/v1".to_string());
        update_environment(&conn, &env).unwrap();

        let envs = get_environments(&conn).unwrap();
        assert_eq!(
            envs[0].default_endpoint,
            Some("https://api.example.com/v1".to_string())
        );
    }

    #[test]
    fn delete_existing_environment() {
        let conn = setup_test_db();
        let env = create_environment(&conn, "to-delete").unwrap();
        delete_environment(&conn, env.id).unwrap();

        let envs = get_environments(&conn).unwrap();
        assert!(envs.is_empty());
    }

    #[test]
    fn delete_nonexistent_environment_does_not_fail() {
        let conn = setup_test_db();
        let result = delete_environment(&conn, 999);
        assert!(result.is_ok());
    }

    #[test]
    fn environment_display() {
        let env = Environment {
            id: 1,
            name: "my-env".to_string(),
            variables: vec![],
            default_endpoint: None,
        };
        assert_eq!(env.to_string(), "my-env");
    }

    #[test]
    fn environment_clone_and_eq() {
        let env = Environment {
            id: 1,
            name: "clone-test".to_string(),
            variables: vec![("k".to_string(), "v".to_string())],
            default_endpoint: None,
        };
        let cloned = env.clone();
        assert_eq!(env, cloned);
    }

    #[test]
    fn save_and_get_request_history() {
        let conn = setup_test_db();
        conn.execute(
            "CREATE TABLE IF NOT EXISTS request_history (
                id INTEGER PRIMARY KEY,
                method TEXT NOT NULL,
                url TEXT NOT NULL,
                status INTEGER,
                duration_ms INTEGER,
                timestamp TEXT NOT NULL
            )",
            [],
        )
        .unwrap();

        save_request_history(&conn, "GET", "https://example.com", Some(200), Some(150)).unwrap();
        save_request_history(&conn, "POST", "https://api.test.com", Some(201), Some(300)).unwrap();

        let history = get_request_history(&conn, 10).unwrap();
        assert_eq!(history.len(), 2);
        assert_eq!(history[0].method, "POST");
        assert_eq!(history[1].method, "GET");
    }

    #[test]
    fn delete_request_history_clears_all() {
        let conn = setup_test_db();
        conn.execute(
            "CREATE TABLE IF NOT EXISTS request_history (
                id INTEGER PRIMARY KEY,
                method TEXT NOT NULL,
                url TEXT NOT NULL,
                status INTEGER,
                duration_ms INTEGER,
                timestamp TEXT NOT NULL
            )",
            [],
        )
        .unwrap();

        save_request_history(&conn, "GET", "https://example.com", Some(200), Some(100)).unwrap();
        delete_request_history(&conn).unwrap();
        let history = get_request_history(&conn, 10).unwrap();
        assert!(history.is_empty());
    }

    #[test]
    fn request_history_limit() {
        let conn = setup_test_db();
        conn.execute(
            "CREATE TABLE IF NOT EXISTS request_history (
                id INTEGER PRIMARY KEY,
                method TEXT NOT NULL,
                url TEXT NOT NULL,
                status INTEGER,
                duration_ms INTEGER,
                timestamp TEXT NOT NULL
            )",
            [],
        )
        .unwrap();

        for i in 0..5 {
            save_request_history(&conn, "GET", &format!("https://example.com/{}", i), Some(200), Some(100)).unwrap();
        }

        let history = get_request_history(&conn, 3).unwrap();
        assert_eq!(history.len(), 3);
    }

    #[test]
    fn create_and_get_collection() {
        let conn = setup_test_db();
        let col = create_collection(&conn, "My API", Some("All endpoints")).unwrap();
        assert_eq!(col.name, "My API");
        assert_eq!(col.description, Some("All endpoints".to_string()));

        let cols = get_collections(&conn).unwrap();
        assert_eq!(cols.len(), 1);
        assert_eq!(cols[0].name, "My API");
    }

    #[test]
    fn create_multiple_collections() {
        let conn = setup_test_db();
        create_collection(&conn, "API v1", None).unwrap();
        create_collection(&conn, "API v2", None).unwrap();
        create_collection(&conn, "Auth", None).unwrap();

        let cols = get_collections(&conn).unwrap();
        assert_eq!(cols.len(), 3);
        let names: Vec<&str> = cols.iter().map(|c| c.name.as_str()).collect();
        assert!(names.contains(&"API v1"));
        assert!(names.contains(&"API v2"));
        assert!(names.contains(&"Auth"));
    }

    #[test]
    fn update_collection_test() {
        let conn = setup_test_db();
        let mut col = create_collection(&conn, "Old Name", None).unwrap();
        col.name = "New Name".to_string();
        col.description = Some("Updated desc".to_string());
        update_collection(&conn, &col).unwrap();

        let cols = get_collections(&conn).unwrap();
        assert_eq!(cols[0].name, "New Name");
        assert_eq!(cols[0].description, Some("Updated desc".to_string()));
    }

    #[test]
    fn delete_collection_test() {
        let conn = setup_test_db();
        let col = create_collection(&conn, "To Delete", None).unwrap();
        delete_collection(&conn, col.id).unwrap();

        let cols = get_collections(&conn).unwrap();
        assert!(cols.is_empty());
    }

    #[test]
    fn create_and_get_folders() {
        let conn = setup_test_db();
        let col = create_collection(&conn, "API", None).unwrap();
        let f1 = create_folder(&conn, col.id, "Auth", None).unwrap();
        let _f2 = create_folder(&conn, col.id, "Users", None).unwrap();
        let f3 = create_folder(&conn, col.id, "Login", Some(f1.id)).unwrap();

        let folders = get_folders(&conn, col.id).unwrap();
        assert_eq!(folders.len(), 3);
        let names: Vec<&str> = folders.iter().map(|f| f.name.as_str()).collect();
        assert!(names.contains(&"Auth"));
        assert!(names.contains(&"Users"));
        assert!(names.contains(&"Login"));

        let login_folder = folders.iter().find(|f| f.name == "Login").unwrap();
        assert_eq!(login_folder.parent_folder_id, Some(f1.id));

        let auth_folder = folders.iter().find(|f| f.name == "Auth").unwrap();
        assert_eq!(auth_folder.parent_folder_id, None);
    }

    #[test]
    fn rename_folder_test() {
        let conn = setup_test_db();
        let col = create_collection(&conn, "API", None).unwrap();
        let folder = create_folder(&conn, col.id, "Old", None).unwrap();
        rename_folder(&conn, folder.id, "New").unwrap();

        let folders = get_folders(&conn, col.id).unwrap();
        assert_eq!(folders[0].name, "New");
    }

    #[test]
    fn delete_folder_cascade() {
        let conn = setup_test_db();
        let col = create_collection(&conn, "API", None).unwrap();
        let folder = create_folder(&conn, col.id, "ToDelete", None).unwrap();
        delete_folder(&conn, folder.id).unwrap();

        let folders = get_folders(&conn, col.id).unwrap();
        assert!(folders.is_empty());
    }

    #[test]
    fn save_and_get_collection_requests() {
        let conn = setup_test_db();
        let col = create_collection(&conn, "API", None).unwrap();
        let headers = vec![("Content-Type".to_string(), "application/json".to_string())];
        let params = vec![("key".to_string(), "value".to_string())];

        save_collection_request(
            &conn, col.id, None, "Get Todos", "GET",
            "https://jsonplaceholder.typicode.com/todos",
            &headers, None, "text", "none", None, &params, None,
        ).unwrap();
        save_collection_request(
            &conn, col.id, None, "Create Todo", "POST",
            "https://jsonplaceholder.typicode.com/todos",
            &headers, Some(r#"{"title":"test"}"#), "text", "bearer", Some("token123"), &[], None,
        ).unwrap();

        let reqs = get_collection_requests(&conn, col.id, None).unwrap();
        assert_eq!(reqs.len(), 2);
        assert_eq!(reqs[0].name, "Get Todos");
        assert_eq!(reqs[1].name, "Create Todo");
        assert_eq!(reqs[0].headers.len(), 1);
        assert_eq!(reqs[1].body, Some(r#"{"title":"test"}"#.to_string()));
        assert_eq!(reqs[1].auth_type, "bearer");
    }

    #[test]
    fn save_request_in_folder() {
        let conn = setup_test_db();
        let col = create_collection(&conn, "API", None).unwrap();
        let folder = create_folder(&conn, col.id, "Auth", None).unwrap();

        save_collection_request(
            &conn, col.id, Some(folder.id), "Login", "POST",
            "https://api.example.com/login",
            &[], Some(r#"{"user":"admin"}"#), "text", "none", None, &[], None,
        ).unwrap();

        let root_reqs = get_collection_requests(&conn, col.id, None).unwrap();
        assert!(root_reqs.is_empty());

        let folder_reqs = get_collection_requests(&conn, col.id, Some(folder.id)).unwrap();
        assert_eq!(folder_reqs.len(), 1);
        assert_eq!(folder_reqs[0].name, "Login");
    }

    #[test]
    fn rename_and_move_collection_request() {
        let conn = setup_test_db();
        let col = create_collection(&conn, "API", None).unwrap();
        let folder = create_folder(&conn, col.id, "Folder", None).unwrap();

        let req = save_collection_request(
            &conn, col.id, None, "Old Name", "GET", "https://example.com",
            &[], None, "text", "none", None, &[], None,
        ).unwrap();

        rename_collection_request(&conn, req.id, "New Name").unwrap();
        move_collection_request(&conn, req.id, Some(folder.id)).unwrap();

        let root_reqs = get_collection_requests(&conn, col.id, None).unwrap();
        assert!(root_reqs.is_empty());

        let folder_reqs = get_collection_requests(&conn, col.id, Some(folder.id)).unwrap();
        assert_eq!(folder_reqs[0].name, "New Name");
    }

    #[test]
    fn delete_collection_request_test() {
        let conn = setup_test_db();
        let col = create_collection(&conn, "API", None).unwrap();
        let req = save_collection_request(
            &conn, col.id, None, "To Delete", "DELETE", "https://example.com/1",
            &[], None, "text", "none", None, &[], None,
        ).unwrap();

        delete_collection_request(&conn, req.id).unwrap();
        let reqs = get_collection_requests(&conn, col.id, None).unwrap();
        assert!(reqs.is_empty());
    }
}
