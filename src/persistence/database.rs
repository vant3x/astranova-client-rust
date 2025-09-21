use directories::ProjectDirs;
use rusqlite::{Connection, Result};
use serde::{Deserialize, Serialize};
use serde_json;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Environment {
    pub id: i32,
    pub name: String,
    pub variables: Vec<(String, String)>,
}

impl std::fmt::Display for Environment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

fn get_db_path() -> PathBuf {
    let proj_dirs = ProjectDirs::from("com", "astranova", "client").unwrap();
    let data_dir = proj_dirs.data_dir();
    std::fs::create_dir_all(data_dir).unwrap();
    data_dir.join("astranova.db")
}

pub fn init() -> Result<Connection> {
    let db_path = get_db_path();
    let conn = Connection::open(db_path)?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS environments (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL UNIQUE,
            variables TEXT NOT NULL
        )",
        [],
    )?;
    Ok(conn)
}

pub fn create_environment(conn: &Connection, name: &str) -> Result<Environment> {
    let variables: Vec<(String, String)> = Vec::new();
    let variables_json = serde_json::to_value(&variables).unwrap();
    conn.execute(
        "INSERT INTO environments (name, variables) VALUES (?1, ?2)",
        &[name, &variables_json.to_string()],
    )?;
    let id = conn.last_insert_rowid();
    Ok(Environment {
        id: id as i32,
        name: name.to_string(),
        variables,
    })
}

pub fn get_environments(conn: &Connection) -> Result<Vec<Environment>> {
    let mut stmt = conn.prepare("SELECT id, name, variables FROM environments")?;
    let env_iter = stmt.query_map([], |row| {
        let variables_json: String = row.get(2)?;
        let variables: Vec<(String, String)> = serde_json::from_str(&variables_json).unwrap();
        Ok(Environment {
            id: row.get(0)?,
            name: row.get(1)?,
            variables,
        })
    })?;

    let mut environments = Vec::new();
    for env in env_iter {
        environments.push(env?);
    }
    Ok(environments)
}

pub fn update_environment(conn: &Connection, env: &Environment) -> Result<()> {
    let variables_json = serde_json::to_value(&env.variables).unwrap();
    conn.execute(
        "UPDATE environments SET name = ?1, variables = ?2 WHERE id = ?3",
        &[&env.name, &variables_json.to_string(), &env.id.to_string()],
    )?;
    Ok(())
}

pub fn delete_environment(conn: &Connection, id: i32) -> Result<()> {
    conn.execute("DELETE FROM environments WHERE id = ?1", &[&id.to_string()])?;
    Ok(())
}