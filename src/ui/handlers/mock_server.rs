use crate::ui::app::{AstraioApp, Message};
use crate::ui::views::mock_server_view;
use iced::Task;
pub fn handle_message(app: &mut AstraioApp, msg: mock_server_view::Message) -> Task<Message> {
    match msg {
        mock_server_view::Message::ToggleAddServer => {
            app.mock_server_view.show_add_server = !app.mock_server_view.show_add_server;
            if !app.mock_server_view.show_add_server {
                app.mock_server_view.new_server_name.clear();
            }
        }
        mock_server_view::Message::NewServerNameChanged(name) => {
            app.mock_server_view.new_server_name = name;
        }
        mock_server_view::Message::CreateServer(name) => {
            if name.trim().is_empty() {
                return Task::none();
            }
            let port = find_free_port();
            match crate::services::mock_server_service::create_and_refresh(
                &app.db_conn,
                name.trim(),
                port,
            ) {
                Ok(servers) => {
                    app.mock_server_view.sync_servers(&servers);
                    app.mock_server_view.new_server_name = String::new();
                    app.mock_server_view.show_add_server = false;
                    app.toast_manager.success(format!(
                        "Created '{}' on port {}",
                        name.trim(),
                        port
                    ));
                }
                Err(e) => {
                    log::error!("Error creating mock server: {}", e);
                    app.toast_manager.error(format!("Failed to create: {}", e));
                }
            }
        }
        mock_server_view::Message::SelectServer(id) => {
            app.mock_server_view.selected_server_id = id;
            app.mock_server_view.endpoint_edit = None;
        }
        mock_server_view::Message::DeleteServer(id) => {
            if let Some(handle) = app.mock_server_handles.remove(&id) {
                crate::protocols::mock_server::stop_mock_server(handle);
                app.mock_server_view.statuses.remove(&id);
            }
            match crate::services::mock_server_service::delete_and_refresh(&app.db_conn, id) {
                Ok(servers) => {
                    app.mock_server_view.sync_servers(&servers);
                    if app.mock_server_view.selected_server_id == Some(id) {
                        app.mock_server_view.selected_server_id = None;
                    }
                    app.toast_manager.success("Mock server deleted");
                }
                Err(e) => log::error!("Error deleting mock server: {}", e),
            }
        }
        mock_server_view::Message::StartServer(id) => {
            let config = match app.mock_server_view.servers.iter().find(|s| s.id == id) {
                Some(c) => c.clone(),
                None => return Task::none(),
            };

            log::info!(
                "[Mock] Starting server '{}' on port {}",
                config.name,
                config.port
            );
            app.mock_server_view.statuses.insert(
                id,
                crate::protocols::mock_server::MockServerStatus::Starting,
            );

            let server_id = id;
            return Task::perform(
                async move { crate::protocols::mock_server::start_mock_server(&config).await },
                move |result| match result {
                    Ok((handle, actual_port)) => {
                        log::info!("[Mock] Server started successfully on port {}", actual_port);
                        Message::MockServerStarted(server_id, handle, actual_port)
                    }
                    Err(e) => {
                        log::error!("[Mock] Failed to start server: {}", e);
                        Message::MockServerStartError(server_id, e)
                    }
                },
            );
        }
        mock_server_view::Message::StopServer(id) => {
            if let Some(handle) = app.mock_server_handles.remove(&id) {
                crate::protocols::mock_server::stop_mock_server(handle);
            }
            app.mock_server_view
                .statuses
                .insert(id, crate::protocols::mock_server::MockServerStatus::Stopped);
            app.toast_manager.info("Mock server stopped");
        }
        mock_server_view::Message::AddEndpoint(server_id) => {
            let body = r#"{"message": "Hello, World!"}"#.to_string();
            app.mock_server_view.endpoint_edit = Some(mock_server_view::EndpointEditState {
                mock_server_id: server_id,
                endpoint_id: None,
                method: "GET".to_string(),
                path: "/".to_string(),
                status: "200".to_string(),
                body: body.clone(),
                delay_ms: "0".to_string(),
            });
        }
        mock_server_view::Message::EditEndpoint(endpoint_id) => {
            let server_id = app.mock_server_view.selected_server_id.unwrap_or(0);
            if let Some(server) = app
                .mock_server_view
                .servers
                .iter()
                .find(|s| s.id == server_id)
            {
                if let Some(ep) = server.endpoints.iter().find(|e| e.id == endpoint_id) {
                    let body = ep.body.clone().unwrap_or_default();
                    app.mock_server_view.endpoint_edit =
                        Some(mock_server_view::EndpointEditState {
                            mock_server_id: server_id,
                            endpoint_id: Some(ep.id),
                            method: ep.method.clone(),
                            path: ep.path.clone(),
                            status: ep.status.to_string(),
                            body: body.clone(),
                            delay_ms: ep.delay_ms.to_string(),
                        });
                }
            }
        }
        mock_server_view::Message::EndpointMethodSelected(method) => {
            if let Some(ref mut edit) = app.mock_server_view.endpoint_edit {
                edit.method = method;
            }
        }
        mock_server_view::Message::EndpointPathChanged(path) => {
            if let Some(ref mut edit) = app.mock_server_view.endpoint_edit {
                edit.path = path;
            }
        }
        mock_server_view::Message::EndpointStatusChanged(status) => {
            if let Some(ref mut edit) = app.mock_server_view.endpoint_edit {
                edit.status = status;
            }
        }
        mock_server_view::Message::EndpointBodyChanged(body) => {
            if let Some(ref mut edit) = app.mock_server_view.endpoint_edit {
                edit.body = body;
            }
        }
        mock_server_view::Message::EndpointDelayChanged(delay) => {
            if let Some(ref mut edit) = app.mock_server_view.endpoint_edit {
                edit.delay_ms = delay;
            }
        }
        mock_server_view::Message::SaveEndpoint => {
            if let Some(edit) = app.mock_server_view.endpoint_edit.take() {
                let status_code: u16 = edit.status.parse().unwrap_or(200);
                let body_opt = if edit.body.is_empty() {
                    None
                } else {
                    Some(edit.body.as_str())
                };

                let result = if let Some(ep_id) = edit.endpoint_id {
                    crate::services::mock_server_service::update_endpoint(
                        &app.db_conn,
                        ep_id,
                        edit.mock_server_id,
                        &edit.method,
                        &edit.path,
                        status_code,
                        body_opt,
                    )
                } else {
                    crate::services::mock_server_service::add_endpoint(
                        &app.db_conn,
                        edit.mock_server_id,
                        &edit.method,
                        &edit.path,
                        status_code,
                        body_opt,
                    )
                };

                match result {
                    Ok(_) => {
                        let servers = crate::services::mock_server_service::get_all(&app.db_conn)
                            .unwrap_or_default();
                        app.mock_server_view.sync_servers(&servers);
                        app.toast_manager.success("Endpoint saved");
                    }
                    Err(e) => {
                        log::error!("Error saving endpoint: {}", e);
                        app.toast_manager.error(format!("Failed to save: {}", e));
                    }
                }
            }
        }
        mock_server_view::Message::CancelEndpointEdit => {
            app.mock_server_view.endpoint_edit = None;
        }
        mock_server_view::Message::DeleteEndpoint(endpoint_id, server_id) => {
            match crate::services::mock_server_service::delete_endpoint(
                &app.db_conn,
                endpoint_id,
                server_id,
            ) {
                Ok(_) => {
                    let servers = crate::services::mock_server_service::get_all(&app.db_conn)
                        .unwrap_or_default();
                    app.mock_server_view.sync_servers(&servers);
                    app.toast_manager.success("Endpoint deleted");
                }
                Err(e) => log::error!("Error deleting endpoint: {}", e),
            }
        }
        mock_server_view::Message::EndpointSearchChanged(query) => {
            app.mock_server_view.endpoint_search = query;
        }
        mock_server_view::Message::ClearLogs => {
            app.mock_server_view.logs.clear();
        }
    }

    Task::none()
}

fn find_free_port() -> u16 {
    use std::net::TcpListener;
    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to find free port");
    listener.local_addr().unwrap().port()
}
