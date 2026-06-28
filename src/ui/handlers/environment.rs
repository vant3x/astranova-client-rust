use crate::ui::app::{AstraNovaApp, Message};
use crate::ui::views::environment_manager;
use iced::Task;

pub fn handle_message(app: &mut AstraNovaApp, msg: environment_manager::Message) -> Task<Message> {
    app.env_manager_view.update(msg.clone());
    match msg {
        environment_manager::Message::CreateEnvironment => {
            let name = app.env_manager_view.new_environment_name.clone();
            match crate::services::environment_service::create_and_refresh(&app.db_conn, &name) {
                Ok(environments) => {
                    let new_env = environments.last().cloned();
                    app.environments = environments;
                    app.env_manager_view.environments = app.environments.clone();
                    app.env_manager_view.new_environment_name = String::new();
                    if let Some(env) = new_env {
                        app.env_manager_view.selected_environment = Some(env);
                    }
                }
                Err(e) => log::error!("Error creating environment: {}", e),
            }
        }
        environment_manager::Message::SaveEnvironment => {
            if let Some(env) = &app.env_manager_view.selected_environment {
                match crate::services::environment_service::save_and_refresh(&app.db_conn, env) {
                    Ok(environments) => {
                        app.environments = environments;
                        app.env_manager_view.environments = app.environments.clone();
                        if let Some(selected_env) = &app.env_manager_view.selected_environment {
                            app.env_manager_view.selected_environment = app
                                .environments
                                .iter()
                                .find(|e| e.id == selected_env.id)
                                .cloned();
                        }
                    }
                    Err(e) => log::error!("Error saving environment: {}", e),
                }
            }
        }
        environment_manager::Message::ConfirmDeleteEnvironment(_env_id) => {
            if let Some(env) = &app.env_manager_view.selected_environment {
                match crate::services::environment_service::delete_and_refresh(
                    &app.db_conn,
                    env.id,
                ) {
                    Ok(environments) => {
                        app.environments = environments;
                        app.env_manager_view.environments = app.environments.clone();
                    }
                    Err(e) => log::error!("Error deleting environment: {}", e),
                }
            }
        }
        environment_manager::Message::LoadEnvFile => {
            return Task::perform(
                async {
                    let file = rfd::AsyncFileDialog::new().pick_file().await;
                    if let Some(file_handle) = file {
                        let data = file_handle.read().await;
                        let mut vars = Vec::new();
                        if let Ok(content) = std::str::from_utf8(&data) {
                            for line in content.lines() {
                                let trimmed_line = line.trim();
                                if trimmed_line.starts_with('#') || trimmed_line.is_empty() {
                                    continue;
                                }
                                if let Some((key, value)) = trimmed_line.split_once('=') {
                                    vars.push((
                                        key.trim().to_string(),
                                        value.trim().to_string(),
                                    ));
                                }
                            }
                        }
                        Some(vars)
                    } else {
                        None
                    }
                },
                Message::EnvFileLoaded,
            );
        }
        environment_manager::Message::Close => {
            app.current_view = crate::ui::app::View::Main;
        }
        _ => (),
    }
    Task::none()
}

pub fn handle_file_loaded(
    app: &mut AstraNovaApp,
    vars: Option<Vec<(String, String)>>,
) -> Task<Message> {
    if let Some(vars) = vars {
        app.env_manager_view
            .update(environment_manager::Message::UpdateVariables(vars));
    }
    Task::none()
}
