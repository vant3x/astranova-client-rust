use crate::ui::app::{AstraioApp, Message};
use crate::ui::views::history_view;
use iced::Task;

pub fn handle_message(app: &mut AstraioApp, msg: history_view::Message) -> Task<Message> {
    match msg.clone() {
        history_view::Message::ConfirmClearHistory => {
            if let Err(e) = crate::services::history_service::clear(&app.db_conn) {
                log::error!("Failed to clear history: {e}");
            }
            app.history_view.update(msg);
        }
        history_view::Message::ResendEntry(entry_id) => {
            if let Ok(Some(entry)) =
                crate::services::history_service::get_by_id(&app.db_conn, entry_id)
            {
                if let Some(new_view) =
                    crate::services::request_restoration::build_view_from_history(&entry)
                {
                    if let Some(active_tab) = app.request_tabs.get_mut(app.active_request_tab_index)
                    {
                        active_tab.url_input = new_view.url_input;
                        active_tab.method = new_view.method;
                        active_tab.headers_editor = new_view.headers_editor;
                        active_tab.body_input = new_view.body_input;
                        active_tab.body_type = new_view.body_type;
                        active_tab.auth = new_view.auth;
                        active_tab.params_editor = new_view.params_editor;
                        active_tab.form_entries = new_view.form_entries;
                        active_tab.multipart_entries = new_view.multipart_entries;
                        active_tab.request_status = crate::ui::request_status::RequestStatus::Idle;
                        active_tab.last_response = None;
                    } else {
                        app.request_tabs.push(new_view);
                        app.active_request_tab_index = app.request_tabs.len() - 1;
                    }
                }
            }
            app.history_view.update(msg);
        }
        history_view::Message::ConfirmDeleteEntry(entry_id) => {
            let _ = crate::services::history_service::delete_entry(&app.db_conn, entry_id);
            app.history_view.update(msg);
        }
        history_view::Message::SearchChanged(_) => {
            app.history_view.update(msg);
            refresh_history_entries(app);
        }
        history_view::Message::FilterMethod(_) => {
            app.history_view.update(msg);
            refresh_history_entries(app);
        }
        history_view::Message::ViewResponse(_) | history_view::Message::CloseResponse => {
            app.history_view.update(msg);
        }
        history_view::Message::ExportHistory => {
            let entries: Vec<crate::persistence::database::RequestHistoryEntry> =
                app.history_view.entries.clone();
            return Task::perform(
                async move {
                    let file = rfd::AsyncFileDialog::new()
                        .add_filter("HAR (HTTP Archive)", &["har"])
                        .add_filter("JSON", &["json"])
                        .add_filter("CSV", &["csv"])
                        .save_file()
                        .await;

                    if let Some(path) = file {
                        let path = path.path();
                        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("har");

                        let content = if ext == "csv" {
                            export_csv(&entries)
                        } else if ext == "json" {
                            export_json(&entries)
                        } else {
                            crate::export::har::export_history_to_har(&entries)
                        };

                        let path_buf = path.to_path_buf();
                        match tokio::fs::write(&path_buf, &content).await {
                            Ok(()) => Ok(format!(
                                "Exported {} entries to {}",
                                entries.len(),
                                ext.to_uppercase()
                            )),
                            Err(e) => Err(format!("Export failed: {e}")),
                        }
                    } else {
                        Err("Export cancelled".to_string())
                    }
                },
                |result| match result {
                    Ok(msg) => Message::HistoryExportComplete(Some(msg)),
                    Err(msg) => Message::HistoryExportComplete(Some(msg)),
                },
            );
        }
        _ => {
            app.history_view.update(msg);
        }
    }
    Task::none()
}

fn refresh_history_entries(app: &mut AstraioApp) {
    let query = app.history_view.search_query.clone();
    let method = app.history_view.filter_method.clone();
    app.history_view.entries =
        crate::services::history_service::search(&app.db_conn, &query, &method, 500)
            .unwrap_or_else(|e| {
                log::error!("Failed to search history: {e}");
                Vec::new()
            });
}

fn export_json(entries: &[crate::persistence::database::RequestHistoryEntry]) -> String {
    serde_json::to_string_pretty(entries).unwrap_or_else(|_| "[]".to_string())
}

fn export_csv(entries: &[crate::persistence::database::RequestHistoryEntry]) -> String {
    let mut wtr = csv::Writer::from_writer(vec![]);
    let _ = wtr.write_record([
        "id",
        "method",
        "url",
        "status",
        "duration_ms",
        "timestamp",
        "request_data",
        "response_data",
    ]);
    for e in entries {
        let request_data = e.request_data.clone().unwrap_or_default();
        let response_data = e.response_data.clone().unwrap_or_default();
        let _ = wtr.write_record([
            &e.id.to_string(),
            &e.method,
            &e.url,
            &e.status.map(|s| s.to_string()).unwrap_or_default(),
            &e.duration_ms.map(|d| d.to_string()).unwrap_or_default(),
            &e.timestamp,
            &request_data,
            &response_data,
        ]);
    }
    String::from_utf8(wtr.into_inner().unwrap_or_default()).unwrap_or_default()
}
