use crate::ui::app::{AstraNovaApp, Message};
use crate::ui::views::collection_view;
use iced::Task;

pub fn handle_message(app: &mut AstraNovaApp, msg: collection_view::Message) -> Task<Message> {
    match msg.clone() {
        collection_view::Message::NewCollectionNameChanged(name) => {
            app.collection_view.new_collection_name = name;
        }
        collection_view::Message::CreateCollection => {
            let name = app.collection_view.new_collection_name.clone();
            if !name.is_empty() {
                match crate::services::collection_service::create_and_refresh(&app.db_conn, &name) {
                    Ok(cols) => {
                        app.collection_view.sync_collections(&cols);
                        app.collection_view.new_collection_name.clear();
                    }
                    Err(e) => log::error!("Error creating collection: {}", e),
                }
            }
        }
        collection_view::Message::SelectCollection(idx) => {
            app.collection_view.panel_state = collection_view::PanelState::CollectionDetail(idx);
            if let Some(col) = app.collection_view.collections.get(idx) {
                let col_id = col.id;
                let folders =
                    crate::services::collection_service::get_folders(&app.db_conn, col_id);
                app.collection_view.sync_folders(&folders);
                let reqs =
                    crate::services::collection_service::get_requests(&app.db_conn, col_id, None);
                app.collection_view.sync_requests(&reqs);
            }
        }
        collection_view::Message::SelectFolder(folder_id) => {
            if let collection_view::PanelState::CollectionDetail(col_idx) =
                app.collection_view.panel_state
            {
                app.collection_view.panel_state =
                    collection_view::PanelState::FolderDetail(col_idx, folder_id);
                if let Some(col) = app.collection_view.collections.get(col_idx) {
                    let reqs = crate::services::collection_service::get_requests(
                        &app.db_conn,
                        col.id,
                        Some(folder_id),
                    );
                    app.collection_view.sync_requests(&reqs);
                }
            }
        }
        collection_view::Message::Close => {
            app.collection_view.panel_state = collection_view::PanelState::Collections;
        }
        collection_view::Message::NewFolderNameChanged(_col_id, name) => {
            app.collection_view.new_folder_name = name;
        }
        collection_view::Message::CreateFolder(col_id) => {
            let name = app.collection_view.new_folder_name.clone();
            if !name.is_empty() {
                match crate::services::collection_service::create_folder_and_refresh(
                    &app.db_conn,
                    col_id,
                    &name,
                ) {
                    Ok(folders) => {
                        app.collection_view.sync_folders(&folders);
                        app.collection_view.new_folder_name.clear();
                    }
                    Err(e) => log::error!("Error creating folder: {}", e),
                }
            }
        }
        collection_view::Message::DeleteCollection(_idx) => {}
        collection_view::Message::ConfirmDeleteCollection(idx) => {
            if let Some(col) = app.collection_view.collections.get(idx) {
                match crate::services::collection_service::delete_and_refresh(&app.db_conn, col.id)
                {
                    Ok(cols) => app.collection_view.sync_collections(&cols),
                    Err(e) => log::error!("Error deleting collection: {}", e),
                }
            }
        }
        collection_view::Message::DeleteFolder(_folder_id) => {}
        collection_view::Message::ConfirmDeleteFolder(folder_id) => {
            if let collection_view::PanelState::CollectionDetail(col_idx) =
                app.collection_view.panel_state
            {
                if let Some(col) = app.collection_view.collections.get(col_idx) {
                    match crate::services::collection_service::delete_folder_and_refresh(
                        &app.db_conn,
                        col.id,
                        folder_id,
                    ) {
                        Ok(folders) => app.collection_view.sync_folders(&folders),
                        Err(e) => log::error!("Error deleting folder: {}", e),
                    }
                }
            }
        }
        collection_view::Message::ImportCollection => {
            app.collection_view.update(msg);
            return Task::perform(
                async move {
                    let file = rfd::AsyncFileDialog::new()
                        .add_filter("Postman Collection", &["json"])
                        .pick_file()
                        .await;
                    if let Some(file_handle) = file {
                        let data = file_handle.read().await;
                        if let Ok(content) = std::str::from_utf8(&data) {
                            return Some(content.to_string());
                        }
                    }
                    None
                },
                |result| {
                    Message::CollectionMsg(collection_view::Message::ImportCollectionData(result))
                },
            );
        }
        collection_view::Message::ImportCollectionData(Some(json)) => {
            match crate::import::postman::parse_postman_collection(&json) {
                Ok(imported) => {
                    match crate::services::collection_service::create_and_refresh(
                        &app.db_conn,
                        &imported.name,
                    ) {
                        Ok(cols) => {
                            if let Some(new_col) = cols.last() {
                                for folder in &imported.folders {
                                    match crate::services::collection_service::create_folder(
                                        &app.db_conn,
                                        new_col.id,
                                        &folder.name,
                                    ) {
                                        Ok(created_folder) => {
                                            for req in &folder.requests {
                                                let _ = crate::services::collection_service::save_request(
                                                    &app.db_conn,
                                                    new_col.id,
                                                    Some(created_folder.id),
                                                    &req.name,
                                                    &req.method,
                                                    &req.url,
                                                    &req.headers,
                                                    req.body.as_deref(),
                                                    "text",
                                                    "none",
                                                    None,
                                                    &req.params,
                                                    None,
                                                );
                                            }
                                        }
                                        Err(e) => log::error!("Error creating folder: {}", e),
                                    }
                                }
                                for req in &imported.requests {
                                    let _ = crate::services::collection_service::save_request(
                                        &app.db_conn,
                                        new_col.id,
                                        None,
                                        &req.name,
                                        &req.method,
                                        &req.url,
                                        &req.headers,
                                        req.body.as_deref(),
                                        "text",
                                        "none",
                                        None,
                                        &req.params,
                                        None,
                                    );
                                }
                                let cols =
                                    crate::services::collection_service::get_all(&app.db_conn);
                                app.collection_view.sync_collections(&cols);
                            }
                        }
                        Err(e) => log::error!("Error creating collection: {}", e),
                    }
                }
                Err(e) => log::error!("Error parsing Postman collection: {}", e),
            }
        }
        collection_view::Message::ImportCollectionData(None) => {}
        collection_view::Message::ImportOpenApi => {
            app.collection_view.update(msg);
            return Task::perform(
                async move {
                    let file = rfd::AsyncFileDialog::new()
                        .add_filter("OpenAPI / Swagger", &["json", "yaml", "yml"])
                        .pick_file()
                        .await;
                    if let Some(file_handle) = file {
                        let data = file_handle.read().await;
                        if let Ok(content) = std::str::from_utf8(&data) {
                            return Some(content.to_string());
                        }
                    }
                    None
                },
                |result| {
                    Message::CollectionMsg(collection_view::Message::ImportOpenApiData(result))
                },
            );
        }
        collection_view::Message::ImportOpenApiData(Some(content)) => {
            let parse_result = if content.trim_start().starts_with('{') {
                crate::openapi::parse_spec(&content)
            } else {
                crate::openapi::parse_spec_from_yaml(&content)
            };

            match parse_result {
                Ok(spec) => {
                    let collection_id = app
                        .db_conn
                        .query_row(
                            "SELECT COALESCE(MAX(id), 0) + 1 FROM collections",
                            [],
                            |row| row.get::<_, i32>(0),
                        )
                        .unwrap_or(1);

                    let generated = crate::openapi::generate_collection(&spec, collection_id);

                    match crate::services::collection_service::create_and_refresh(
                        &app.db_conn,
                        &generated.collection.name,
                    ) {
                        Ok(cols) => {
                            if let Some(new_col) = cols.last() {
                                for folder in &generated.folders {
                                    match crate::services::collection_service::create_folder(
                                        &app.db_conn,
                                        new_col.id,
                                        &folder.name,
                                    ) {
                                        Ok(created_folder) => {
                                            for req in &generated.requests {
                                                let _ = crate::services::collection_service::save_request(
                                                    &app.db_conn,
                                                    new_col.id,
                                                    Some(created_folder.id),
                                                    &req.name,
                                                    &req.method,
                                                    &req.url,
                                                    &req.headers,
                                                    req.body.as_deref(),
                                                    "text",
                                                    "none",
                                                    None,
                                                    &req.params,
                                                    None,
                                                );
                                            }
                                        }
                                        Err(e) => log::error!("Error creating folder: {}", e),
                                    }
                                }
                                for req in &generated.requests {
                                    let _ = crate::services::collection_service::save_request(
                                        &app.db_conn,
                                        new_col.id,
                                        None,
                                        &req.name,
                                        &req.method,
                                        &req.url,
                                        &req.headers,
                                        req.body.as_deref(),
                                        "text",
                                        "none",
                                        None,
                                        &req.params,
                                        None,
                                    );
                                }
                                let cols =
                                    crate::services::collection_service::get_all(&app.db_conn);
                                app.collection_view.sync_collections(&cols);
                                app.toast_manager.success(format!(
                                    "Imported {} endpoints from OpenAPI spec",
                                    generated.requests.len()
                                ));
                            }
                        }
                        Err(e) => {
                            log::error!("Error creating collection: {}", e);
                            app.toast_manager.error(format!("Import failed: {}", e));
                        }
                    }
                }
                Err(e) => {
                    log::error!("Error parsing OpenAPI spec: {}", e);
                    app.toast_manager
                        .error(format!("Invalid OpenAPI spec: {}", e));
                }
            }
        }
        collection_view::Message::ImportOpenApiData(None) => {}
        collection_view::Message::ExportCollection(idx) => {
            if let Some(col) = app.collection_view.collections.get(idx) {
                let folders =
                    crate::services::collection_service::get_folders(&app.db_conn, col.id);
                let requests =
                    crate::services::collection_service::get_requests(&app.db_conn, col.id, None);
                match crate::export::postman::export_collection(col, &folders, &requests) {
                    Ok(json) => {
                        let col_name = col.name.clone();
                        app.collection_view.update(msg);
                        return Task::perform(
                            async move {
                                let file = rfd::AsyncFileDialog::new()
                                    .add_filter("Postman Collection", &["json"])
                                    .set_file_name(&format!("{}.json", col_name))
                                    .save_file()
                                    .await;
                                if let Some(file_handle) = file {
                                    let path = file_handle.path().to_path_buf();
                                    let _ = tokio::fs::write(&path, json.as_bytes()).await;
                                }
                                None::<()>
                            },
                            |_: Option<_>| {
                                Message::CollectionMsg(
                                    collection_view::Message::ExportCollectionData(String::new()),
                                )
                            },
                        );
                    }
                    Err(e) => log::error!("Error exporting collection: {}", e),
                }
            }
        }
        collection_view::Message::ExportCollectionData(_) => {}
        collection_view::Message::ConfirmRenameCollection => {
            if let Some(idx) = app.collection_view.renaming_collection {
                let new_name = app.collection_view.rename_collection_value.clone();
                if let Some(col) = app.collection_view.collections.get(idx) {
                    match crate::services::collection_service::rename(&app.db_conn, col, &new_name)
                    {
                        Ok(()) => {
                            let cols = crate::services::collection_service::get_all(&app.db_conn);
                            app.collection_view.sync_collections(&cols);
                        }
                        Err(e) => log::error!("Error renaming collection: {}", e),
                    }
                }
            }
        }
        collection_view::Message::ConfirmRenameFolder => {
            if let Some(folder_id) = app.collection_view.renaming_folder {
                let new_name = app.collection_view.rename_folder_value.clone();
                match crate::services::collection_service::rename_folder(
                    &app.db_conn,
                    folder_id,
                    &new_name,
                ) {
                    Ok(()) => {
                        if let collection_view::PanelState::CollectionDetail(col_idx) =
                            app.collection_view.panel_state
                        {
                            if let Some(col) = app.collection_view.collections.get(col_idx) {
                                let folders = crate::services::collection_service::get_folders(
                                    &app.db_conn,
                                    col.id,
                                );
                                app.collection_view.sync_folders(&folders);
                            }
                        }
                    }
                    Err(e) => log::error!("Error renaming folder: {}", e),
                }
            }
        }
        collection_view::Message::ConfirmRenameRequest => {
            if let Some(req_id) = app.collection_view.renaming_request {
                let new_name = app.collection_view.rename_request_value.clone();
                match crate::services::collection_service::rename_request(
                    &app.db_conn,
                    req_id,
                    &new_name,
                ) {
                    Ok(()) => {
                        refresh_requests_after_rename(app);
                    }
                    Err(e) => log::error!("Error renaming request: {}", e),
                }
            }
        }
        collection_view::Message::DeleteRequest(_req_id) => {}
        collection_view::Message::ConfirmDeleteRequest(req_id) => {
            handle_delete_request(app, req_id);
        }
        collection_view::Message::LoadRequest(req_id) => {
            load_collection_request(app, req_id);
        }
        collection_view::Message::SaveCurrentRequest => {
            save_current_to_collection(app);
        }
        _ => {}
    }
    app.collection_view.update(msg);
    Task::none()
}

fn refresh_requests_after_rename(app: &mut AstraNovaApp) {
    if let collection_view::PanelState::CollectionDetail(col_idx) = app.collection_view.panel_state
    {
        if let Some(col) = app.collection_view.collections.get(col_idx) {
            let reqs =
                crate::services::collection_service::get_requests(&app.db_conn, col.id, None);
            app.collection_view.sync_requests(&reqs);
        }
    } else if let collection_view::PanelState::FolderDetail(col_idx, folder_id) =
        app.collection_view.panel_state
    {
        if let Some(col) = app.collection_view.collections.get(col_idx) {
            let reqs = crate::services::collection_service::get_requests(
                &app.db_conn,
                col.id,
                Some(folder_id),
            );
            app.collection_view.sync_requests(&reqs);
        }
    }
}

fn handle_delete_request(app: &mut AstraNovaApp, req_id: i32) {
    if let collection_view::PanelState::CollectionDetail(col_idx) = app.collection_view.panel_state
    {
        if let Some(col) = app.collection_view.collections.get(col_idx) {
            match crate::services::collection_service::delete_request_and_refresh(
                &app.db_conn,
                col.id,
                None,
                req_id,
            ) {
                Ok(reqs) => app.collection_view.sync_requests(&reqs),
                Err(e) => log::error!("Error deleting request: {}", e),
            }
        }
    } else if let collection_view::PanelState::FolderDetail(col_idx, folder_id) =
        app.collection_view.panel_state
    {
        if let Some(col) = app.collection_view.collections.get(col_idx) {
            match crate::services::collection_service::delete_request_and_refresh(
                &app.db_conn,
                col.id,
                Some(folder_id),
                req_id,
            ) {
                Ok(reqs) => app.collection_view.sync_requests(&reqs),
                Err(e) => log::error!("Error deleting request: {}", e),
            }
        }
    }
}

fn load_collection_request(app: &mut AstraNovaApp, req_id: i32) {
    let conn = &app.db_conn;
    let all_reqs = match &app.collection_view.panel_state {
        collection_view::PanelState::CollectionDetail(idx) => {
            if let Some(col) = app.collection_view.collections.get(*idx) {
                crate::services::collection_service::get_requests(conn, col.id, None)
            } else {
                return;
            }
        }
        collection_view::PanelState::FolderDetail(_col_idx, _folder_id) => {
            app.collection_view.requests.clone()
        }
        _ => return,
    };

    let req = match all_reqs.iter().find(|r| r.id == req_id) {
        Some(r) => r.clone(),
        None => return,
    };

    let new_view = crate::services::request_restoration::build_view_from_collection_request(&req);
    app.request_tabs.push(new_view);
    app.active_request_tab_index = app.request_tabs.len() - 1;
}

fn save_current_to_collection(app: &mut AstraNovaApp) {
    if let Some(view) = app.request_tabs.get(app.active_request_tab_index) {
        let col_id = match app.collection_view.selected_collection_id {
            Some(id) => id,
            None => {
                if let Some(col) = app.collection_view.collections.first() {
                    col.id
                } else {
                    return;
                }
            }
        };

        let request = view.build_request();
        let auth_type = match &view.auth {
            crate::data::auth::Auth::BearerToken(_) => "bearer",
            crate::data::auth::Auth::Basic { .. } => "basic",
            crate::data::auth::Auth::ApiKey { .. } => "api_key",
            crate::data::auth::Auth::Digest { .. } => "digest",
            crate::data::auth::Auth::OAuth2(_) => "oauth2",
            crate::data::auth::Auth::None => "none",
        };
        let auth_data = match &view.auth {
            crate::data::auth::Auth::None => None,
            auth => serde_json::to_string(auth).ok(),
        };

        let params: Vec<(String, String)> = view
            .params_editor
            .entries
            .iter()
            .filter(|p| !p.key.is_empty())
            .map(|p| (p.key.clone(), p.value.clone()))
            .collect();

        let body_type = match view.body_type {
            crate::ui::views::http_request_view::BodyType::Multipart => "multipart",
            _ => "text",
        };

        let name = if request.url.len() > 40 {
            format!("{} {}", request.method, &request.url[..40])
        } else {
            format!("{} {}", request.method, request.url)
        };

        let _ = crate::services::collection_service::save_request(
            &app.db_conn,
            col_id,
            None,
            &name,
            &request.method,
            &request.url,
            &request.headers,
            request.body.as_deref(),
            body_type,
            auth_type,
            auth_data.as_deref(),
            &params,
            None,
        );

        let reqs = crate::services::collection_service::get_requests(&app.db_conn, col_id, None);
        app.collection_view.sync_requests(&reqs);
    }
}
