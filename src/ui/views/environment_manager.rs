use crate::persistence::database::Environment;
use crate::ui::components::key_value_editor::{self, KeyValueEditor};
use iced::{
    widget::{button, column, container, pick_list, row, text, text_input},
    Element, Length, theme,
};
use iced::widget::container as iced_container;

#[derive(Debug, Clone)]
pub enum Message {
    SelectEnvironment(i32),
    EnvironmentNameChanged(String),
    NewEnvironmentNameChanged(String),
    VariablesEditor(key_value_editor::Message),
    CreateEnvironment,
    SaveEnvironment,
    DeleteEnvironment,
    Close,
}

#[derive(Debug, Clone)]
pub struct EnvironmentManagerView {
    pub environments: Vec<Environment>,
    pub selected_environment: Option<Environment>,
    pub new_environment_name: String,
    pub variables_editor: KeyValueEditor,
}

impl EnvironmentManagerView {
    pub fn new(environments: Vec<Environment>) -> Self {
        Self {
            environments,
            selected_environment: None,
            new_environment_name: String::new(),
            variables_editor: KeyValueEditor::new("Add Variable".to_string()),
        }
    }

    pub fn update(&mut self, message: Message) {
        match message {
            Message::SelectEnvironment(id) => {
                self.selected_environment = self.environments.iter().find(|e| e.id == id).cloned();
                if let Some(env) = &self.selected_environment {
                    self.variables_editor.entries = env
                        .variables
                        .iter()
                        .enumerate()
                        .map(|(i, (k, v))| key_value_editor::KeyValueEntry {
                            id: i,
                            key: k.clone(),
                            value: v.clone(),
                        })
                        .collect();
                }
            }
            Message::EnvironmentNameChanged(name) => {
                if let Some(env) = &mut self.selected_environment {
                    env.name = name;
                }
            }
            Message::NewEnvironmentNameChanged(name) => {
                self.new_environment_name = name;
            }
            Message::VariablesEditor(msg) => self.variables_editor.update(msg),
            Message::CreateEnvironment => {
                // This message is handled in app.rs
            }
            Message::SaveEnvironment => {
                if let Some(env) = &mut self.selected_environment {
                    env.variables = self
                        .variables_editor
                        .entries
                        .iter()
                        .map(|e| (e.key.clone(), e.value.clone()))
                        .collect();
                }
            }
            Message::DeleteEnvironment => {
                self.selected_environment = None;
            }
            Message::Close => {}
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let environments_list = pick_list(
            &self.environments[..],
            self.selected_environment.as_ref(),
            |environment| Message::SelectEnvironment(environment.id),
        )
        .placeholder("Select an environment");

        let mut environment_details = column![];
        if let Some(selected_env) = &self.selected_environment {
            environment_details = environment_details
                .push(text_input("Name", &selected_env.name).on_input(Message::EnvironmentNameChanged))
                .push(self.variables_editor.view().map(Message::VariablesEditor))
                .push(button("Save").on_press(Message::SaveEnvironment))
                .push(button("Delete").on_press(Message::DeleteEnvironment));
        }

        let create_new_env_section = column![
            text_input(
                "New Environment Name",
                &self.new_environment_name
            )
            .on_input(Message::NewEnvironmentNameChanged),
            button("Create").on_press(Message::CreateEnvironment)
        ];

        let content = column![
            row![text("Environments"), environments_list].spacing(10),
            environment_details,
            create_new_env_section,
            button("Close").on_press(Message::Close),
        ]
        .spacing(20)
        .padding(20);

        iced_container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(|_theme| iced_container::Style::default())
            .into()
    }
}