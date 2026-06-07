use crate::persistence::database::{self, Environment};
use rusqlite::Connection;

pub fn get_all(conn: &Connection) -> Vec<Environment> {
    database::get_environments(conn).unwrap_or_default()
}

pub fn create(conn: &Connection, name: &str) -> Result<Environment, String> {
    database::create_environment(conn, name).map_err(|e| e.to_string())
}

pub fn update(conn: &Connection, env: &Environment) -> Result<(), String> {
    database::update_environment(conn, env).map_err(|e| e.to_string())
}

pub fn delete(conn: &Connection, id: i32) -> Result<(), String> {
    database::delete_environment(conn, id).map_err(|e| e.to_string())
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
    fn create_and_list_environment() {
        let conn = setup_test_db();
        let env = create(&conn, "dev").unwrap();
        assert_eq!(env.name, "dev");

        let envs = get_all(&conn);
        assert_eq!(envs.len(), 1);
    }

    #[test]
    fn update_environment() {
        let conn = setup_test_db();
        let mut env = create(&conn, "dev").unwrap();
        env.variables = vec![("URL".to_string(), "http://localhost".to_string())];
        update(&conn, &env).unwrap();

        let envs = get_all(&conn);
        assert_eq!(envs[0].variables.len(), 1);
    }

    #[test]
    fn delete_environment() {
        let conn = setup_test_db();
        let env = create(&conn, "dev").unwrap();
        delete(&conn, env.id).unwrap();

        let envs = get_all(&conn);
        assert!(envs.is_empty());
    }
}
