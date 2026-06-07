use crate::persistence::database::{self, Collection, CollectionFolder, CollectionRequest};
use rusqlite::Connection;

pub fn get_all(conn: &Connection) -> Vec<Collection> {
    database::get_collections(conn).unwrap_or_default()
}

pub fn create(conn: &Connection, name: &str) -> Result<Collection, String> {
    database::create_collection(conn, name, None).map_err(|e| e.to_string())
}

pub fn delete(conn: &Connection, id: i32) {
    let _ = database::delete_collection(conn, id);
}

pub fn get_folders(conn: &Connection, collection_id: i32) -> Vec<CollectionFolder> {
    database::get_folders(conn, collection_id).unwrap_or_default()
}

pub fn create_folder(
    conn: &Connection,
    collection_id: i32,
    name: &str,
) -> Result<CollectionFolder, String> {
    database::create_folder(conn, collection_id, name, None).map_err(|e| e.to_string())
}

pub fn delete_folder(conn: &Connection, id: i32) {
    let _ = database::delete_folder(conn, id);
}

pub fn get_requests(
    conn: &Connection,
    collection_id: i32,
    folder_id: Option<i32>,
) -> Vec<CollectionRequest> {
    database::get_collection_requests(conn, collection_id, folder_id).unwrap_or_default()
}

pub fn save_request(
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
) -> Result<CollectionRequest, String> {
    database::save_collection_request(
        conn,
        collection_id,
        folder_id,
        name,
        method,
        url,
        headers,
        body,
        body_type,
        auth_type,
        auth_data,
        params,
        config_json,
    )
    .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_test_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
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
    fn create_and_list_collection() {
        let conn = setup_test_db();
        let col = create(&conn, "My API").unwrap();
        assert_eq!(col.name, "My API");

        let cols = get_all(&conn);
        assert_eq!(cols.len(), 1);
    }

    #[test]
    fn create_and_list_folder() {
        let conn = setup_test_db();
        let col = create(&conn, "API").unwrap();
        let folder = create_folder(&conn, col.id, "Auth").unwrap();
        assert_eq!(folder.name, "Auth");

        let folders = get_folders(&conn, col.id);
        assert_eq!(folders.len(), 1);
    }

    #[test]
    fn save_and_get_request() {
        let conn = setup_test_db();
        let col = create(&conn, "API").unwrap();
        let req = save_request(
            &conn,
            col.id,
            None,
            "Get Todos",
            "GET",
            "https://api.example.com/todos",
            &[],
            None,
            "text",
            "none",
            None,
            &[],
            None,
        )
        .unwrap();
        assert_eq!(req.name, "Get Todos");

        let reqs = get_requests(&conn, col.id, None);
        assert_eq!(reqs.len(), 1);
    }
}
