use crate::ui::app::{AstraioApp, Message};
use crate::ui::views::collection_view;
use iced::Task;

pub fn handle_message(app: &mut AstraioApp, msg: collection_view::Message) -> Task<Message> {
    match msg.clone() {
        collection_view::Message::ToggleCollection(idx) => {
            if let Some(col) = app.collection_view.collections.get(idx) {
                let col_id = col.id;
                let is_expanding = app
                    .collection_view
                    .expanded_collections
                    .get(idx)
                    .is_some_and(|e| !*e);
                app.collection_view.update(msg);
                if is_expanding {
                    refresh_collection_data(app, col_id);
                }
                return Task::none();
            }
        }
        collection_view::Message::ToggleFolder(folder_id) => {
            app.collection_view.update(msg.clone());
            if let Some(folder) = app
                .collection_view
                .folders
                .iter()
                .find(|f| f.id == folder_id)
            {
                let col_id = folder.collection_id;
                let is_expanding = app
                    .collection_view
                    .folder_index(folder_id)
                    .and_then(|f_idx| app.collection_view.expanded_folders.get(f_idx))
                    .is_some_and(|e| !*e);
                if is_expanding {
                    refresh_collection_data(app, col_id);
                }
            }
            return Task::none();
        }
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
                        refresh_all_data(app);
                    }
                    Err(e) => log::error!("Error creating collection: {}", e),
                }
            }
        }
        collection_view::Message::CreateFolder => {
            let name = app.collection_view.new_folder_name.clone();
            let col_id = app.collection_view.new_folder_target;
            let parent_id = app.collection_view.new_folder_parent;
            if let Some(col_id) = col_id {
                if !name.is_empty() {
                    match crate::services::collection_service::create_folder_with_parent(
                        &app.db_conn,
                        col_id,
                        &name,
                        parent_id,
                    ) {
                        Ok(_) => {
                            app.collection_view.new_folder_name.clear();
                            app.collection_view.new_folder_target = None;
                            app.collection_view.new_folder_parent = None;
                            refresh_collection_data(app, col_id);
                        }
                        Err(e) => log::error!("Error creating folder: {}", e),
                    }
                }
            }
        }
        collection_view::Message::HideNewFolderInput => {
            app.collection_view.new_folder_target = None;
            app.collection_view.new_folder_parent = None;
            app.collection_view.new_folder_name.clear();
        }
        collection_view::Message::NewFolderNameChanged(name) => {
            app.collection_view.new_folder_name = name;
        }
        collection_view::Message::ConfirmDeleteCollection(idx) => {
            if let Some(col) = app.collection_view.collections.get(idx) {
                let col_id = col.id;
                match crate::services::collection_service::delete_and_refresh(&app.db_conn, col_id)
                {
                    Ok(cols) => {
                        app.collection_view.sync_collections(&cols);
                        refresh_all_data(app);
                    }
                    Err(e) => log::error!("Error deleting collection: {}", e),
                }
            }
        }
        collection_view::Message::ConfirmDeleteFolder(folder_id) => {
            if let Some(folder) = app
                .collection_view
                .folders
                .iter()
                .find(|f| f.id == folder_id)
            {
                let col_id = folder.collection_id;
                match crate::services::collection_service::delete_folder_and_refresh(
                    &app.db_conn,
                    col_id,
                    folder_id,
                ) {
                    Ok(_) => {
                        refresh_collection_data(app, col_id);
                    }
                    Err(e) => log::error!("Error deleting folder: {}", e),
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
                                                    &crate::persistence::database::SaveRequestParams::imported(
                                                        new_col.id,
                                                        Some(created_folder.id),
                                                        &req.name,
                                                        &req.method,
                                                        &req.url,
                                                        &req.headers,
                                                        req.body.as_deref(),
                                                        &req.params,
                                                        req.scripts.as_deref(),
                                                    ),
                                                );
                                            }
                                        }
                                        Err(e) => log::error!("Error creating folder: {}", e),
                                    }
                                }
                                for req in &imported.requests {
                                    let _ = crate::services::collection_service::save_request(
                                        &app.db_conn,
                                        &crate::persistence::database::SaveRequestParams::imported(
                                            new_col.id,
                                            None,
                                            &req.name,
                                            &req.method,
                                            &req.url,
                                            &req.headers,
                                            req.body.as_deref(),
                                            &req.params,
                                            req.scripts.as_deref(),
                                        ),
                                    );
                                }
                                refresh_all_data(app);
                            }
                        }
                        Err(e) => log::error!("Error creating collection: {}", e),
                    }
                }
                Err(e) => log::error!("Error parsing Postman collection: {}", e),
            }
        }
        collection_view::Message::ImportCollectionData(None) => {}
        collection_view::Message::ImportHar => {
            app.collection_view.update(msg);
            return Task::perform(
                async move {
                    let file = rfd::AsyncFileDialog::new()
                        .add_filter("HTTP Archive (HAR)", &["har", "json"])
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
                |result| Message::CollectionMsg(collection_view::Message::ImportHarData(result)),
            );
        }
        collection_view::Message::ImportHarData(Some(json)) => {
            match crate::import::har::parse_har_collection(&json) {
                Ok(imported) => {
                    match crate::services::collection_service::create_and_refresh(
                        &app.db_conn,
                        &imported.name,
                    ) {
                        Ok(cols) => {
                            if let Some(new_col) = cols.last() {
                                let mut requests_count = 0;
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
                                                    &crate::persistence::database::SaveRequestParams::imported(
                                                        new_col.id,
                                                        Some(created_folder.id),
                                                        &req.name,
                                                        &req.method,
                                                        &req.url,
                                                        &req.headers,
                                                        req.body.as_deref(),
                                                        &req.params,
                                                        None,
                                                    ),
                                                );
                                                requests_count += 1;
                                            }
                                        }
                                        Err(e) => log::error!("Error creating folder: {}", e),
                                    }
                                }
                                for req in &imported.requests {
                                    let _ = crate::services::collection_service::save_request(
                                        &app.db_conn,
                                        &crate::persistence::database::SaveRequestParams::imported(
                                            new_col.id,
                                            None,
                                            &req.name,
                                            &req.method,
                                            &req.url,
                                            &req.headers,
                                            req.body.as_deref(),
                                            &req.params,
                                            None,
                                        ),
                                    );
                                    requests_count += 1;
                                }
                                refresh_all_data(app);
                                app.toast_manager.success(format!(
                                    "Imported {} requests from HAR into collection '{}'",
                                    requests_count, imported.name
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
                    log::error!("Error parsing HAR collection: {}", e);
                    app.toast_manager.error(format!("Invalid HAR file: {}", e));
                }
            }
        }
        collection_view::Message::ImportHarData(None) => {}
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
                                let mut folder_id_map: std::collections::HashMap<String, i32> =
                                    std::collections::HashMap::new();

                                for folder in &generated.folders {
                                    match crate::services::collection_service::create_folder(
                                        &app.db_conn,
                                        new_col.id,
                                        &folder.name,
                                    ) {
                                        Ok(created_folder) => {
                                            folder_id_map
                                                .insert(folder.name.clone(), created_folder.id);
                                        }
                                        Err(e) => log::error!("Error creating folder: {}", e),
                                    }
                                }

                                for req in &generated.requests {
                                    let folder_id = req
                                        .folder_id
                                        .and_then(|fid| {
                                            generated.folders.iter().find(|f| f.id == fid).map(
                                                |f| {
                                                    folder_id_map
                                                        .get(&f.name)
                                                        .copied()
                                                        .unwrap_or(fid)
                                                },
                                            )
                                        })
                                        .or_else(|| {
                                            req.folder_name
                                                .as_ref()
                                                .and_then(|name| folder_id_map.get(name).copied())
                                        });

                                    let _ = crate::services::collection_service::save_request(
                                        &app.db_conn,
                                        &crate::persistence::database::SaveRequestParams {
                                            collection_id: new_col.id,
                                            folder_id,
                                            name: req.name.clone(),
                                            method: req.method.clone(),
                                            url: req.url.clone(),
                                            headers: req.headers.clone(),
                                            body: req.body.clone(),
                                            body_type: crate::persistence::database::CollectionBodyType::Text,
                                            auth_type: crate::persistence::database::CollectionAuthType::None,
                                            auth_data: None,
                                            params: req.params.clone(),
                                            config_json: None,
                                            scripts: None,
                                        },
                                    );
                                }
                                refresh_all_data(app);
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
                    crate::services::collection_service::get_folders(&app.db_conn, col.id)
                        .unwrap_or_default();
                let requests =
                    crate::services::collection_service::get_requests(&app.db_conn, col.id, None)
                        .unwrap_or_default();
                match crate::export::postman::export_collection(col, &folders, &requests) {
                    Ok(json) => {
                        let col_name = col.name.clone();
                        app.collection_view.update(msg);
                        return Task::perform(
                            async move {
                                let file = rfd::AsyncFileDialog::new()
                                    .add_filter("Postman Collection", &["json"])
                                    .set_file_name(format!("{}.json", col_name))
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
                                    collection_view::Message::ExportCollectionData(()),
                                )
                            },
                        );
                    }
                    Err(e) => log::error!("Error exporting collection: {}", e),
                }
            }
        }
        collection_view::Message::ExportCollectionData(_) => {}
        collection_view::Message::ExportCollectionHar(idx) => {
            if let Some(col) = app.collection_view.collections.get(idx) {
                let requests =
                    crate::services::collection_service::get_requests(&app.db_conn, col.id, None)
                        .unwrap_or_default();
                let har_json = crate::export::har::export_collection_to_har(col, &requests);
                let col_name = col.name.clone();
                app.collection_view.update(msg);
                return Task::perform(
                    async move {
                        let file = rfd::AsyncFileDialog::new()
                            .add_filter("HTTP Archive (HAR)", &["har"])
                            .set_file_name(format!("{}.har", col_name))
                            .save_file()
                            .await;
                        if let Some(file_handle) = file {
                            let path = file_handle.path().to_path_buf();
                            let _ = tokio::fs::write(&path, har_json.as_bytes()).await;
                        }
                        None::<()>
                    },
                    |_: Option<_>| {
                        Message::CollectionMsg(
                            collection_view::Message::ExportCollectionHarData(()),
                        )
                    },
                );
            }
        }
        collection_view::Message::ExportCollectionHarData(_) => {}
        collection_view::Message::ConfirmRenameCollection => {
            if let Some(idx) = app.collection_view.renaming_collection {
                let new_name = app.collection_view.rename_collection_value.clone();
                if let Some(col) = app.collection_view.collections.get(idx) {
                    match crate::services::collection_service::rename(&app.db_conn, col, &new_name)
                    {
                        Ok(()) => {
                            refresh_all_data(app);
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
                        if let Some(folder) = app
                            .collection_view
                            .folders
                            .iter()
                            .find(|f| f.id == folder_id)
                        {
                            let col_id = folder.collection_id;
                            refresh_collection_data(app, col_id);
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
                        if let Some(req) =
                            app.collection_view.requests.iter().find(|r| r.id == req_id)
                        {
                            let col_id = req.collection_id;
                            refresh_collection_data(app, col_id);
                        }
                    }
                    Err(e) => log::error!("Error renaming request: {}", e),
                }
            }
        }
        collection_view::Message::ConfirmDeleteRequest(req_id) => {
            if let Some(req) = app.collection_view.requests.iter().find(|r| r.id == req_id) {
                let col_id = req.collection_id;
                let folder_id = req.folder_id;
                match crate::services::collection_service::delete_request_and_refresh(
                    &app.db_conn,
                    col_id,
                    folder_id,
                    req_id,
                ) {
                    Ok(_) => {
                        refresh_collection_data(app, col_id);
                    }
                    Err(e) => log::error!("Error deleting request: {}", e),
                }
            }
        }
        collection_view::Message::LoadRequest(req_id) => {
            load_collection_request(app, req_id);
        }
        collection_view::Message::SaveCurrentRequest => {
            save_current_to_collection(app);
        }
        collection_view::Message::SaveCollectionVariables(col_idx) => {
            app.collection_view.update(msg.clone());
            if let Some(col) = app.collection_view.collections.get(col_idx) {
                let col_id = col.id;
                let variables = col.variables.clone();
                if let Err(e) = crate::services::collection_service::update_variables(
                    &app.db_conn,
                    col_id,
                    &variables,
                ) {
                    log::error!("Failed to save collection variables: {}", e);
                }
            }
            return Task::none();
        }
        collection_view::Message::MoveRequestUp(req_id) => {
            if let Some(req) = app.collection_view.requests.iter().find(|r| r.id == req_id) {
                let req = req.clone();
                match crate::services::collection_service::move_up(&app.db_conn, &req) {
                    Ok(()) => {
                        refresh_collection_data(app, req.collection_id);
                    }
                    Err(e) => log::error!("Error moving request up: {}", e),
                }
            }
        }
        collection_view::Message::MoveRequestDown(req_id) => {
            if let Some(req) = app.collection_view.requests.iter().find(|r| r.id == req_id) {
                let req = req.clone();
                match crate::services::collection_service::move_down(&app.db_conn, &req) {
                    Ok(()) => {
                        refresh_collection_data(app, req.collection_id);
                    }
                    Err(e) => log::error!("Error moving request down: {}", e),
                }
            }
        }
        collection_view::Message::StartMoveToFolder(_req_id) => {
            app.collection_view.update(msg.clone());
            return Task::none();
        }
        collection_view::Message::MoveToFolder(req_id, target_folder_id) => {
            match crate::services::collection_service::move_to_folder(
                &app.db_conn,
                req_id,
                target_folder_id,
            ) {
                Ok(()) => {
                    if let Some(req) = app.collection_view.requests.iter().find(|r| r.id == req_id)
                    {
                        let col_id = req.collection_id;
                        app.collection_view.update(msg.clone());
                        refresh_collection_data(app, col_id);
                        return Task::none();
                    }
                    app.collection_view.update(msg.clone());
                }
                Err(e) => {
                    log::error!("Error moving request to folder: {}", e);
                    app.collection_view.update(msg.clone());
                }
            }
        }
        collection_view::Message::CancelMoveToFolder => {
            app.collection_view.update(msg.clone());
            return Task::none();
        }
        _ => {}
    }
    app.collection_view.update(msg);
    Task::none()
}

fn refresh_collection_data(app: &mut AstraioApp, col_id: i32) {
    let folders =
        crate::services::collection_service::get_folders(&app.db_conn, col_id).unwrap_or_default();
    app.collection_view
        .sync_folders_for_collection(col_id, &folders);
    let reqs = crate::services::collection_service::get_requests(&app.db_conn, col_id, None)
        .unwrap_or_default();
    app.collection_view
        .sync_requests_for_collection(col_id, &reqs);
}

fn refresh_all_data(app: &mut AstraioApp) {
    let cols = crate::services::collection_service::get_all(&app.db_conn).unwrap_or_default();
    app.collection_view.sync_collections(&cols);

    let expanded_indices: Vec<usize> = app
        .collection_view
        .expanded_collections
        .iter()
        .enumerate()
        .filter_map(|(idx, &expanded)| if expanded { Some(idx) } else { None })
        .collect();

    for idx in expanded_indices {
        if let Some(col) = app.collection_view.collections.get(idx) {
            let col_id = col.id;
            refresh_collection_data(app, col_id);
        }
    }
}

fn load_collection_request(app: &mut AstraioApp, req_id: i32) {
    let req = match app.collection_view.requests.iter().find(|r| r.id == req_id) {
        Some(r) => r.clone(),
        None => return,
    };

    if req.body_type == crate::persistence::database::CollectionBodyType::Graphql {
        load_graphql_from_collection(app, &req);
        return;
    }

    let new_view = crate::services::request_restoration::build_view_from_collection_request(&req);
    app.request_tabs.push(new_view);
    app.active_request_tab_index = app.request_tabs.len() - 1;
}

fn load_graphql_from_collection(
    app: &mut AstraioApp,
    req: &crate::persistence::database::CollectionRequest,
) {
    app.graphql_view.url_input = req.url.clone();

    let mut headers_editor = crate::ui::components::key_value_editor::KeyValueEditor::default();
    let entries: Vec<crate::ui::components::key_value_editor::KeyValueEntry> = req
        .headers
        .iter()
        .enumerate()
        .map(
            |(i, (k, v))| crate::ui::components::key_value_editor::KeyValueEntry {
                id: i,
                key: k.clone(),
                value: v.clone(),
                secret: false,
            },
        )
        .collect();
    headers_editor.entries = entries;
    app.graphql_view.headers_editor = headers_editor;

    if let Some(body) = &req.body {
        if let Ok(graphql_request) =
            serde_json::from_str::<crate::protocols::graphql::GraphQLRequest>(body)
        {
            app.graphql_view.query_input =
                iced::widget::text_editor::Content::with_text(&graphql_request.query);
            if let Some(variables) = &graphql_request.variables {
                let vars_str = serde_json::to_string_pretty(variables).unwrap_or_default();
                app.graphql_view.variables_input =
                    iced::widget::text_editor::Content::with_text(&vars_str);
            }
            if let Some(op_name) = &graphql_request.operation_name {
                app.graphql_view.operation_name = op_name.clone();
            }
        } else {
            app.graphql_view.query_input = iced::widget::text_editor::Content::with_text(body);
        }
    }

    if let Some(data) = &req.auth_data {
        if data.starts_with('{') {
            if let Ok(auth) = serde_json::from_str::<crate::data::auth::Auth>(data) {
                app.graphql_view.auth = auth;
            }
        }
    }

    app.active_protocol = crate::ui::app::Protocol::GraphQL;
}

fn save_current_to_collection(app: &mut AstraioApp) {
    let col_id = match &app.collection_view.selected_item {
        Some(collection_view::TreeItemId::Collection(idx)) => {
            app.collection_view.collections.get(*idx).map(|c| c.id)
        }
        Some(collection_view::TreeItemId::Folder(folder_id)) => app
            .collection_view
            .folders
            .iter()
            .find(|f| f.id == *folder_id)
            .map(|f| f.collection_id),
        Some(collection_view::TreeItemId::Request(req_id)) => app
            .collection_view
            .requests
            .iter()
            .find(|r| r.id == *req_id)
            .map(|r| r.collection_id),
        None => app.collection_view.collections.first().map(|c| c.id),
    };

    let col_id = match col_id {
        Some(id) => id,
        None => {
            match crate::services::collection_service::create_and_refresh(
                &app.db_conn,
                "My Collection",
            ) {
                Ok(cols) => {
                    if let Some(new_col) = cols.last() {
                        let new_id = new_col.id;
                        refresh_all_data(app);
                        new_id
                    } else {
                        return;
                    }
                }
                Err(e) => {
                    log::error!("Failed to create default collection: {}", e);
                    return;
                }
            }
        }
    };

    let folder_id = match &app.collection_view.selected_item {
        Some(collection_view::TreeItemId::Folder(folder_id)) => Some(*folder_id),
        Some(collection_view::TreeItemId::Request(req_id)) => app
            .collection_view
            .requests
            .iter()
            .find(|r| r.id == *req_id)
            .and_then(|r| r.folder_id),
        _ => None,
    };

    let view = match app.request_tabs.get(app.active_request_tab_index) {
        Some(v) => v,
        None => return,
    };

    let request = match view.build_request() {
        Ok(r) => r,
        Err(_) => return,
    };
    let auth_type = match &view.auth {
        crate::data::auth::Auth::BearerToken(_) => {
            crate::persistence::database::CollectionAuthType::Bearer
        }
        crate::data::auth::Auth::Basic { .. } => {
            crate::persistence::database::CollectionAuthType::Basic
        }
        crate::data::auth::Auth::ApiKey { .. } => {
            crate::persistence::database::CollectionAuthType::ApiKey
        }
        crate::data::auth::Auth::Digest { .. } => {
            crate::persistence::database::CollectionAuthType::Digest
        }
        crate::data::auth::Auth::OAuth2(_) => {
            crate::persistence::database::CollectionAuthType::Oauth2
        }
        crate::data::auth::Auth::None => crate::persistence::database::CollectionAuthType::None,
    };
    let auth_data = match &view.auth {
        crate::data::auth::Auth::None => None,
        auth => auth.to_safe_json().ok(),
    };

    let params: Vec<(String, String)> = view
        .params_editor
        .entries
        .iter()
        .filter(|p| !p.key.is_empty())
        .map(|p| (p.key.clone(), p.value.clone()))
        .collect();

    let body_type = match view.body_type {
        crate::ui::views::http_request_view::BodyType::Multipart => {
            crate::persistence::database::CollectionBodyType::Multipart
        }
        crate::ui::views::http_request_view::BodyType::FormUrlencoded => {
            crate::persistence::database::CollectionBodyType::FormUrlencoded
        }
        _ => crate::persistence::database::CollectionBodyType::Text,
    };

    let name = if request.url.len() > 40 {
        format!(
            "{} {}",
            request.method,
            request.url.chars().take(40).collect::<String>()
        )
    } else {
        format!("{} {}", request.method, request.url)
    };

    let scripts_json = view
        .parse_scripts_from_editors()
        .ok()
        .and_then(|s| s.to_json().ok());

    let config_json = if view.request_config == crate::http_client::config::RequestConfig::default()
    {
        None
    } else {
        serde_json::to_string(&view.request_config).ok()
    };

    let _ = crate::services::collection_service::save_request(
        &app.db_conn,
        &crate::persistence::database::SaveRequestParams {
            collection_id: col_id,
            folder_id,
            name,
            method: request.method.to_string(),
            url: request.url,
            headers: request.headers,
            body: request.body,
            body_type,
            auth_type,
            auth_data,
            params,
            config_json,
            scripts: scripts_json,
        },
    );

    refresh_collection_data(app, col_id);
}
