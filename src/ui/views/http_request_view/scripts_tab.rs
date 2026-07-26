use super::{HttpRequestView, Message, ScriptTab};
use iced::widget::text_editor;
use iced::widget::{button, column, container, row, scrollable, text};
use iced::{Color, Element, Length, Theme};

impl HttpRequestView {
    pub(super) fn create_scripts_tab_content(&self) -> Element<'_, Message, Theme, iced::Renderer> {
        let pre_request_btn = if self.active_script_tab == ScriptTab::PreRequest {
            button("Pre-request")
                .on_press(Message::ScriptTabSelected(ScriptTab::PreRequest))
                .style(iced::widget::button::primary)
        } else {
            button("Pre-request").on_press(Message::ScriptTabSelected(ScriptTab::PreRequest))
        };

        let post_response_btn = if self.active_script_tab == ScriptTab::PostResponse {
            button("Post-response")
                .on_press(Message::ScriptTabSelected(ScriptTab::PostResponse))
                .style(iced::widget::button::primary)
        } else {
            button("Post-response").on_press(Message::ScriptTabSelected(ScriptTab::PostResponse))
        };

        let output_btn = if self.active_script_tab == ScriptTab::Output {
            button("Output")
                .on_press(Message::ScriptTabSelected(ScriptTab::Output))
                .style(iced::widget::button::primary)
        } else {
            button("Output").on_press(Message::ScriptTabSelected(ScriptTab::Output))
        };

        let tab_buttons = row![pre_request_btn, post_response_btn, output_btn].spacing(5);

        let action_buttons = row![
            button("Copy Scripts").on_press(Message::CopyScripts),
            button("Paste Scripts").on_press(Message::PasteScripts),
            button("Save").on_press(Message::SaveScripts),
        ]
        .spacing(5);

        let editor_content: Element<'_, Message> = match self.active_script_tab {
            ScriptTab::PreRequest => text_editor(&self.pre_request_script_editor)
                .on_action(Message::PreRequestScriptChanged)
                .highlight("javascript", self.highlighter_theme)
                .height(Length::Fill)
                .into(),
            ScriptTab::PostResponse => text_editor(&self.post_response_script_editor)
                .on_action(Message::PostResponseScriptChanged)
                .highlight("javascript", self.highlighter_theme)
                .height(Length::Fill)
                .into(),
            ScriptTab::Output => {
                let mut output_text = String::new();
                if !self.script_output.pre_logs.is_empty() {
                    output_text.push_str("=== Pre-request logs ===\n");
                    for log in &self.script_output.pre_logs {
                        output_text.push_str(&format!("  {}\n", log));
                    }
                    output_text.push('\n');
                }
                if !self.script_output.pre_errors.is_empty() {
                    output_text.push_str("=== Pre-request errors ===\n");
                    for err in &self.script_output.pre_errors {
                        output_text.push_str(&format!("  {}\n", err));
                    }
                    output_text.push('\n');
                }
                if !self.script_output.post_logs.is_empty() {
                    output_text.push_str("=== Post-response logs ===\n");
                    for log in &self.script_output.post_logs {
                        output_text.push_str(&format!("  {}\n", log));
                    }
                    output_text.push('\n');
                }
                if !self.script_output.post_errors.is_empty() {
                    output_text.push_str("=== Post-response errors ===\n");
                    for err in &self.script_output.post_errors {
                        output_text.push_str(&format!("  {}\n", err));
                    }
                    output_text.push('\n');
                }
                if !self.script_output.extracted_vars.is_empty() {
                    output_text.push_str("=== Extracted variables ===\n");
                    for (k, v) in &self.script_output.extracted_vars {
                        let preview: String = v.chars().take(80).collect();
                        output_text.push_str(&format!("  {} = {}\n", k, preview));
                    }
                    output_text.push('\n');
                }
                if !self.script_output.test_results.is_empty() {
                    output_text.push_str("=== Test Results ===\n");
                    let passed = self
                        .script_output
                        .test_results
                        .iter()
                        .filter(|t| t.passed)
                        .count();
                    let total = self.script_output.test_results.len();
                    output_text.push_str(&format!("  {}/{} passed\n\n", passed, total));
                    for test in &self.script_output.test_results {
                        let icon = if test.passed { "PASS" } else { "FAIL" };
                        output_text.push_str(&format!("  [{}] {}", icon, test.name));
                        if let Some(msg) = &test.message {
                            output_text.push_str(&format!(" - {}", msg));
                        }
                        output_text.push('\n');
                    }
                    output_text.push('\n');
                }
                if output_text.is_empty() {
                    output_text =
                        "No script output yet. Send a request to see results.".to_string();
                }
                scrollable(text(output_text).size(12))
                    .height(Length::Fill)
                    .into()
            }
        };

        let help_text = text("JavaScript API: pm.environment.get/set(name, value), pm.request.getUrl/setUrl/setMethod/setBody/setHeader/getHeader/removeHeader, pm.response.status/body/headers/json/url/method/responseTime/size, pm.log(...), pm.test(name, fn), pm.expect(value).toBe/toBeTruthy/toContain/toBeGreaterThan/toBeLessThan/toHaveLength. Tokens: Date.now(), Math.*, JSON.*, String.*, Array methods.")
            .size(11)
            .color(Color::from_rgb(0.5, 0.5, 0.5));

        container(
            column![
                text("Scripts").size(16),
                help_text,
                row![tab_buttons, action_buttons].spacing(10),
                editor_content,
            ]
            .spacing(8)
            .padding(5),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }
}
