use crate::error::AppError;
use crate::persistence::database;
use crate::protocols::mock_server::MockServerConfig;
use rusqlite::Connection;

pub fn get_all(conn: &Connection) -> Result<Vec<MockServerConfig>, AppError> {
    Ok(database::get_all_mock_servers(conn)?)
}

pub fn create(conn: &Connection, name: &str, port: u16) -> Result<MockServerConfig, AppError> {
    let id = database::create_mock_server(conn, name, port)?;
    Ok(database::get_mock_server(conn, id)?)
}

#[allow(dead_code)]
pub fn update(
    conn: &Connection,
    id: i32,
    name: &str,
    port: u16,
    enabled: bool,
) -> Result<(), AppError> {
    Ok(database::update_mock_server(conn, id, name, port, enabled)?)
}

pub fn delete(conn: &Connection, id: i32) -> Result<(), AppError> {
    Ok(database::delete_mock_server(conn, id)?)
}

pub fn create_and_refresh(
    conn: &Connection,
    name: &str,
    port: u16,
) -> Result<Vec<MockServerConfig>, AppError> {
    create(conn, name, port)?;
    get_all(conn)
}

pub fn delete_and_refresh(conn: &Connection, id: i32) -> Result<Vec<MockServerConfig>, AppError> {
    delete(conn, id)?;
    get_all(conn)
}

#[allow(clippy::too_many_arguments)]
pub fn add_endpoint(
    conn: &Connection,
    mock_server_id: i32,
    method: &str,
    path: &str,
    status: u16,
    headers: &[(String, String)],
    body: Option<&str>,
    delay_ms: u64,
) -> Result<MockServerConfig, AppError> {
    database::create_mock_endpoint(
        conn,
        mock_server_id,
        method,
        path,
        status,
        headers,
        body,
        delay_ms,
    )?;
    Ok(database::get_mock_server(conn, mock_server_id)?)
}

#[allow(clippy::too_many_arguments)]
pub fn update_endpoint(
    conn: &Connection,
    endpoint_id: i32,
    mock_server_id: i32,
    method: &str,
    path: &str,
    status: u16,
    headers: &[(String, String)],
    body: Option<&str>,
    delay_ms: u64,
) -> Result<MockServerConfig, AppError> {
    database::update_mock_endpoint(
        conn,
        endpoint_id,
        method,
        path,
        status,
        headers,
        body,
        delay_ms,
    )?;
    Ok(database::get_mock_server(conn, mock_server_id)?)
}

pub fn delete_endpoint(
    conn: &Connection,
    endpoint_id: i32,
    mock_server_id: i32,
) -> Result<MockServerConfig, AppError> {
    database::delete_mock_endpoint(conn, endpoint_id)?;
    Ok(database::get_mock_server(conn, mock_server_id)?)
}
