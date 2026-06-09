use crate::persistence::database::{self, Collection, CollectionFolder, CollectionRequest};
use rusqlite::Connection;

pub fn get_all(conn: &Connection) -> Vec<Collection> {
    database::get_collections(conn).unwrap_or_default()
}

pub fn create(conn: &Connection, name: &str) -> Result<Collection, String> {
    database::create_collection(conn, name, None).map_err(|e| e.to_string())
}

pub fn update(conn: &Connection, collection: &Collection) -> Result<(), String> {
    database::update_collection(conn, collection).map_err(|e| e.to_string())
}

pub fn delete(conn: &Connection, id: i32) -> Result<(), String> {
    database::delete_collection(conn, id).map_err(|e| e.to_string())
}

pub fn create_and_refresh(conn: &Connection, name: &str) -> Result<Vec<Collection>, String> {
    create(conn, name)?;
    Ok(get_all(conn))
}

pub fn delete_and_refresh(conn: &Connection, id: i32) -> Result<Vec<Collection>, String> {
    delete(conn, id)?;
    Ok(get_all(conn))
}

pub fn rename(conn: &Connection, collection: &Collection, new_name: &str) -> Result<(), String> {
    let mut updated = collection.clone();
    updated.name = new_name.to_string();
    update(conn, &updated)
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

pub fn delete_folder(conn: &Connection, id: i32) -> Result<(), String> {
    database::delete_folder(conn, id).map_err(|e| e.to_string())
}

pub fn rename_folder(conn: &Connection, id: i32, new_name: &str) -> Result<(), String> {
    database::rename_folder(conn, id, new_name).map_err(|e| e.to_string())
}

pub fn create_folder_and_refresh(
    conn: &Connection,
    collection_id: i32,
    name: &str,
) -> Result<Vec<CollectionFolder>, String> {
    create_folder(conn, collection_id, name)?;
    Ok(get_folders(conn, collection_id))
}

pub fn delete_folder_and_refresh(
    conn: &Connection,
    collection_id: i32,
    folder_id: i32,
) -> Result<Vec<CollectionFolder>, String> {
    delete_folder(conn, folder_id)?;
    Ok(get_folders(conn, collection_id))
}

pub fn get_requests(
    conn: &Connection,
    collection_id: i32,
    folder_id: Option<i32>,
) -> Vec<CollectionRequest> {
    database::get_collection_requests(conn, collection_id, folder_id).unwrap_or_default()
}

#[allow(clippy::too_many_arguments)]
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

pub fn rename_request(conn: &Connection, id: i32, new_name: &str) -> Result<(), String> {
    database::rename_collection_request(conn, id, new_name).map_err(|e| e.to_string())
}

#[allow(dead_code)]
pub fn move_request(conn: &Connection, id: i32, new_folder_id: Option<i32>) -> Result<(), String> {
    database::move_collection_request(conn, id, new_folder_id).map_err(|e| e.to_string())
}

pub fn delete_request(conn: &Connection, id: i32) -> Result<(), String> {
    database::delete_collection_request(conn, id).map_err(|e| e.to_string())
}

pub fn delete_request_and_refresh(
    conn: &Connection,
    collection_id: i32,
    folder_id: Option<i32>,
    request_id: i32,
) -> Result<Vec<CollectionRequest>, String> {
    delete_request(conn, request_id)?;
    Ok(get_requests(conn, collection_id, folder_id))
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

    #[test]
    fn rename_collection_test() {
        let conn = setup_test_db();
        let col = create(&conn, "Old").unwrap();
        rename(&conn, &col, "New").unwrap();

        let cols = get_all(&conn);
        assert_eq!(cols[0].name, "New");
    }

    #[test]
    fn rename_folder_test() {
        let conn = setup_test_db();
        let col = create(&conn, "API").unwrap();
        let folder = create_folder(&conn, col.id, "Old").unwrap();
        rename_folder(&conn, folder.id, "New").unwrap();

        let folders = get_folders(&conn, col.id);
        assert_eq!(folders[0].name, "New");
    }

    #[test]
    fn rename_request_test() {
        let conn = setup_test_db();
        let col = create(&conn, "API").unwrap();
        let req = save_request(
            &conn,
            col.id,
            None,
            "Old",
            "GET",
            "https://example.com",
            &[],
            None,
            "text",
            "none",
            None,
            &[],
            None,
        )
        .unwrap();
        rename_request(&conn, req.id, "New").unwrap();

        let reqs = get_requests(&conn, col.id, None);
        assert_eq!(reqs[0].name, "New");
    }

    #[test]
    fn delete_request_test() {
        let conn = setup_test_db();
        let col = create(&conn, "API").unwrap();
        let req = save_request(
            &conn,
            col.id,
            None,
            "To Delete",
            "DELETE",
            "https://example.com/1",
            &[],
            None,
            "text",
            "none",
            None,
            &[],
            None,
        )
        .unwrap();

        delete_request(&conn, req.id).unwrap();
        let reqs = get_requests(&conn, col.id, None);
        assert!(reqs.is_empty());
    }

    #[test]
    fn move_request_test() {
        let conn = setup_test_db();
        let col = create(&conn, "API").unwrap();
        let folder = create_folder(&conn, col.id, "Auth").unwrap();
        let req = save_request(
            &conn,
            col.id,
            None,
            "Login",
            "POST",
            "https://api.example.com/login",
            &[],
            None,
            "text",
            "none",
            None,
            &[],
            None,
        )
        .unwrap();

        move_request(&conn, req.id, Some(folder.id)).unwrap();
        let root_reqs = get_requests(&conn, col.id, None);
        assert!(root_reqs.is_empty());
        let folder_reqs = get_requests(&conn, col.id, Some(folder.id));
        assert_eq!(folder_reqs.len(), 1);
    }

    #[test]
    fn create_and_refresh_returns_full_list() {
        let conn = setup_test_db();
        let cols = create_and_refresh(&conn, "API v1").unwrap();
        assert_eq!(cols.len(), 1);

        let cols = create_and_refresh(&conn, "API v2").unwrap();
        assert_eq!(cols.len(), 2);
    }

    #[test]
    fn delete_and_refresh_removes_entry() {
        let conn = setup_test_db();
        let col = create(&conn, "API").unwrap();
        create(&conn, "Auth").unwrap();
        let cols = delete_and_refresh(&conn, col.id).unwrap();
        assert_eq!(cols.len(), 1);
    }

    #[test]
    fn create_folder_and_refresh_returns_folders() {
        let conn = setup_test_db();
        let col = create(&conn, "API").unwrap();
        let folders = create_folder_and_refresh(&conn, col.id, "Auth").unwrap();
        assert_eq!(folders.len(), 1);
    }

    #[test]
    fn delete_folder_and_refresh_removes_folder() {
        let conn = setup_test_db();
        let col = create(&conn, "API").unwrap();
        let folder = create_folder(&conn, col.id, "ToDelete").unwrap();
        let folders = delete_folder_and_refresh(&conn, col.id, folder.id).unwrap();
        assert!(folders.is_empty());
    }

    #[test]
    fn delete_request_and_refresh_removes_request() {
        let conn = setup_test_db();
        let col = create(&conn, "API").unwrap();
        let req = save_request(
            &conn,
            col.id,
            None,
            "To Delete",
            "DELETE",
            "https://example.com/1",
            &[],
            None,
            "text",
            "none",
            None,
            &[],
            None,
        )
        .unwrap();
        let reqs = delete_request_and_refresh(&conn, col.id, None, req.id).unwrap();
        assert!(reqs.is_empty());
    }
}
