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

pub fn delete_request_history(conn: &Connection) -> Result<()> {
    conn.execute("DELETE FROM request_history", [])?;
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
}
