use crate::ui::app::{AstraNovaApp, Message};
use crate::ui::views::history_view;
use iced::Task;

pub fn handle_message(app: &mut AstraNovaApp, msg: history_view::Message) -> Task<Message> {
    match msg.clone() {
        history_view::Message::ClearHistory => {
            crate::services::history_service::clear(&app.db_conn);
            app.history_view.update(msg);
        }
        history_view::Message::ResendEntry(entry_id) => {
            if let Some(entry) =
                crate::services::history_service::get_by_id(&app.db_conn, entry_id)
            {
                if let Some(new_view) =
                    crate::services::request_restoration::build_view_from_history(&entry)
                {
                    app.request_tabs.push(new_view);
                    app.active_request_tab_index = app.request_tabs.len() - 1;
                }
            }
            app.history_view.update(msg);
        }
        history_view::Message::SearchChanged(_) => {
            app.history_view.update(msg);
        }
        history_view::Message::FilterMethod(_) => {
            app.history_view.update(msg);
        }
        history_view::Message::ExportHistory => {
            app.history_view.update(msg);
        }
    }
    Task::none()
}
