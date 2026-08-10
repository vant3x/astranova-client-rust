use crate::ai::types::{AiProvider, AiProviderConfig, AiRole};
use crate::ui::theme::ThemeColors;

use iced::{
    widget::{button, column, container, row, rule, scrollable, text, text_input},
    Alignment, Color, Element, Length, Renderer, Theme,
};
use iced_fonts::lucide;

#[derive(Debug, Clone)]
pub enum Message {
    InputChanged(String),
    SendMessage,
    ReceiveStreamStart,
    ReceiveStreamChunk(String),
    ReceiveStreamEnd,
    ReceiveError(String),
    ProviderSelected(AiProvider),
    ModelChanged(String),
    ApiKeyChanged(String),
    BaseUrlChanged(String),
    ToggleSettings,
    SaveProviderConfig,
    SetDefaultProvider(usize),
    DeleteProvider(usize),
    ClearChat,
    CopyMessage(usize),
    ApplyToRequest(String),
    AutoContextToggled(bool),
    SystemPromptChanged(String),
    NewConversation,
    SelectConversation(usize),
    DeleteConversation(usize),
    QuickAction(QuickAction),
    TabChanged(AiTab),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AiTab {
    #[default]
    Chat,
    MockData,
}

impl std::fmt::Display for AiTab {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AiTab::Chat => write!(f, "Chat"),
            AiTab::MockData => write!(f, "Mock Data"),
        }
    }
}

#[derive(Debug, Clone)]
pub enum QuickAction {
    GenerateRequest,
    ExplainResponse,
    GenerateScript,
    GenerateMockData,
    DebugError,
    TransformFormat,
}

impl QuickAction {
    pub fn label(&self) -> &str {
        match self {
            QuickAction::GenerateRequest => "Generate Request",
            QuickAction::ExplainResponse => "Explain Response",
            QuickAction::GenerateScript => "Generate Script",
            QuickAction::GenerateMockData => "Generate Mock Data",
            QuickAction::DebugError => "Debug Error",
            QuickAction::TransformFormat => "Transform",
        }
    }

    pub fn icon(&self) -> Element<'static, Message, Theme, Renderer> {
        match self {
            QuickAction::GenerateRequest => lucide::send().into(),
            QuickAction::ExplainResponse => lucide::info().into(),
            QuickAction::GenerateScript => lucide::code().into(),
            QuickAction::GenerateMockData => lucide::database().into(),
            QuickAction::DebugError => lucide::bug().into(),
            QuickAction::TransformFormat => lucide::repeat().into(),
        }
    }

    pub fn prompt_prefix(&self) -> &str {
        match self {
            QuickAction::GenerateRequest => "Generate an HTTP request for: ",
            QuickAction::ExplainResponse => "Explain this API response and identify any issues: ",
            QuickAction::GenerateScript => "Generate a pre-request JavaScript script that: ",
            QuickAction::GenerateMockData => "Generate realistic mock data for an API endpoint that returns: ",
            QuickAction::DebugError => "Help me debug this API error: ",
            QuickAction::TransformFormat => "Transform this data: ",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ChatBubble {
    pub role: AiRole,
    pub content: String,
    pub timestamp: String,
    pub tokens_used: u32,
    pub is_streaming: bool,
}

#[derive(Debug, Clone)]
pub struct MockDataConfig {
    pub description: String,
    pub count: String,
    pub format: MockFormat,
    pub output: String,
    pub is_generating: bool,
}

impl Default for MockDataConfig {
    fn default() -> Self {
        Self {
            description: String::new(),
            count: "5".to_string(),
            format: MockFormat::Json,
            output: String::new(),
            is_generating: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MockFormat {
    Json,
    Csv,
    Xml,
}

impl std::fmt::Display for MockFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MockFormat::Json => write!(f, "JSON"),
            MockFormat::Csv => write!(f, "CSV"),
            MockFormat::Xml => write!(f, "XML"),
        }
    }
}

impl MockFormat {
    pub fn all() -> &'static [MockFormat] {
        &[MockFormat::Json, MockFormat::Csv, MockFormat::Xml]
    }
}

#[derive(Debug, Clone, Default)]
pub struct AiChatView {
    pub input: String,
    pub messages: Vec<ChatBubble>,
    pub is_streaming: bool,
    pub active_tab: AiTab,
    pub auto_context: bool,
    pub system_prompt: String,
    pub show_settings: bool,
    pub providers: Vec<AiProviderConfig>,
    pub active_provider_index: Option<usize>,
    pub editing_api_key: String,
    pub editing_base_url: String,
    pub editing_model: String,
    pub editing_provider_name: String,
    pub selected_provider_type: AiProvider,
    pub mock_data: MockDataConfig,
    pub streaming_buffer: String,
    pub error_message: Option<String>,
}

impl AiChatView {
    pub fn new() -> Self {
        Self {
            system_prompt: "You are an expert API development assistant. Help generate requests, \
             explain responses, create scripts, and generate mock data for testing."
                .to_string(),
            ..Default::default()
        }
    }

    pub fn add_message(&mut self, role: AiRole, content: String, tokens: u32) {
        let now = chrono::Utc::now().format("%H:%M:%S").to_string();
        self.messages.push(ChatBubble {
            role,
            content,
            timestamp: now,
            tokens_used: tokens,
            is_streaming: false,
        });
    }

    pub fn start_streaming(&mut self) {
        self.is_streaming = true;
        self.streaming_buffer.clear();
        let now = chrono::Utc::now().format("%H:%M:%S").to_string();
        self.messages.push(ChatBubble {
            role: AiRole::Assistant,
            content: String::new(),
            timestamp: now,
            tokens_used: 0,
            is_streaming: true,
        });
    }

    pub fn append_stream_chunk(&mut self, chunk: &str) {
        self.streaming_buffer.push_str(chunk);
        if let Some(last) = self.messages.last_mut() {
            last.content = self.streaming_buffer.clone();
        }
    }

    pub fn finish_streaming(&mut self) {
        self.is_streaming = false;
        if let Some(last) = self.messages.last_mut() {
            last.is_streaming = false;
        }
        self.streaming_buffer.clear();
    }

    pub fn view(&self) -> Element<'_, Message, Theme, Renderer> {
        let header = self.render_header();
        let content = match self.active_tab {
            AiTab::Chat => self.render_chat(),
            AiTab::MockData => self.render_mock_data(),
        };

        column![header, rule::horizontal(1), content]
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn render_header(&self) -> Element<'_, Message, Theme, Renderer> {
        let tabs = row![
            button(text("Chat").size(13))
                .style(if self.active_tab == AiTab::Chat {
                    primary_button_style()
                } else {
                    secondary_button_style()
                })
                .on_press(Message::TabChanged(AiTab::Chat)),
            button(text("Mock Data").size(13))
                .style(if self.active_tab == AiTab::MockData {
                    primary_button_style()
                } else {
                    secondary_button_style()
                })
                .on_press(Message::TabChanged(AiTab::MockData)),
        ]
        .spacing(4);

        let settings_btn = button(
            row![lucide::settings().size(14)].spacing(4)
        )
        .style(ghost_button_style())
        .on_press(Message::ToggleSettings);

        row![
            text("AI Assistant").size(16).font(iced::Font::default()),
            iced::widget::Space::new().width(Length::Fill),
            tabs,
            iced::widget::Space::new().width(Length::Fill),
            settings_btn,
        ]
        .spacing(12)
        .align_y(Alignment::Center)
        .padding(iced::Padding::from([8, 12]))
        .into()
    }

    fn render_chat(&self) -> Element<'_, Message, Theme, Renderer> {
        if self.show_settings {
            return self.render_settings();
        }

        let provider_info = self.render_provider_bar();
        let messages_area = self.render_messages();
        let quick_actions = self.render_quick_actions();
        let input_area = self.render_input();

        column![
            provider_info,
            rule::horizontal(1),
            messages_area,
            quick_actions,
            rule::horizontal(1),
            input_area,
        ]
        .height(Length::Fill)
        .into()
    }

    fn render_provider_bar(&self) -> Element<'_, Message, Theme, Renderer> {
        let provider_label = if let Some(idx) = self.active_provider_index {
            if let Some(config) = self.providers.get(idx) {
                format!("{} / {}", config.provider, config.model)
            } else {
                "No provider configured".to_string()
            }
        } else {
            "No provider configured".to_string()
        };

        let provider_text = text(provider_label).size(12).color(ThemeColors::TEXT_SECONDARY);

        let context_toggle = button(
            row![
                if self.auto_context {
                    lucide::check().size(12)
                } else {
                    lucide::square().size(12)
                },
                text("Context").size(11),
            ]
            .spacing(4),
        )
        .style(ghost_button_style())
        .on_press(Message::AutoContextToggled(!self.auto_context));

        let clear_btn = button(row![lucide::trash().size(12)].spacing(4))
            .style(ghost_button_style())
            .on_press(Message::ClearChat);

        row![provider_text, iced::widget::Space::new().width(Length::Fill), context_toggle, clear_btn]
            .spacing(8)
            .align_y(Alignment::Center)
            .padding(iced::Padding::from([6, 12]))
            .into()
    }

    fn render_messages(&self) -> Element<'_, Message, Theme, Renderer> {
        if self.messages.is_empty() {
            return self.render_empty_state();
        }

        let mut messages_col = column![].spacing(8);

        for (idx, msg) in self.messages.iter().enumerate() {
            let bubble = self.render_bubble(idx, msg);
            messages_col = messages_col.push(bubble);
        }

        scrollable(messages_col.padding(12))
            .height(Length::Fill)
            .into()
    }

    fn render_empty_state(&self) -> Element<'_, Message, Theme, Renderer> {
        let icon = lucide::sparkles().size(48);
        let title = text("AI Assistant").size(20).color(ThemeColors::TEXT_PRIMARY);
        let subtitle = text("Ask me anything about APIs, generate requests, or create mock data")
            .size(13)
            .color(ThemeColors::TEXT_SECONDARY);

        let tips = column![
            tip_row(lucide::send::<Theme, Renderer>().size(12).into(), "Generate HTTP requests from descriptions"),
            tip_row(lucide::info::<Theme, Renderer>().size(12).into(), "Explain API responses"),
            tip_row(lucide::code::<Theme, Renderer>().size(12).into(), "Create pre/post-request scripts"),
            tip_row(lucide::database::<Theme, Renderer>().size(12).into(), "Generate mock data for testing"),
            tip_row(lucide::bug::<Theme, Renderer>().size(12).into(), "Debug API errors"),
        ]
        .spacing(6)
        .padding(iced::Padding::from([16, 0]));

        container(
            column![icon, title, subtitle, tips]
                .spacing(12)
                .align_x(Alignment::Center),
        )
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .into()
    }

    fn render_bubble<'s>(&'s self, idx: usize, msg: &'s ChatBubble) -> Element<'s, Message, Theme, Renderer> {
        let is_user = msg.role == AiRole::User;
        let is_assistant = msg.role == AiRole::Assistant;

        let role_icon = if is_user {
            lucide::user().size(14)
        } else {
            lucide::sparkles().size(14)
        };

        let role_label = text(if is_user { "You" } else { "AI" })
            .size(11)
            .color(if is_user {
                ThemeColors::ACCENT
            } else {
                ThemeColors::PURPLE
            });

        let timestamp = text(&msg.timestamp)
            .size(10)
            .color(ThemeColors::TEXT_DIM);

        let header = row![role_icon, role_label, timestamp].spacing(6).align_y(Alignment::Center);

        let content_text: Element<'_, Message, Theme, Renderer> =
            if msg.content.is_empty() && msg.is_streaming {
            row![
                text("Thinking").size(13).color(ThemeColors::TEXT_SECONDARY),
                text("...").size(13).color(ThemeColors::TEXT_MUTED),
            ]
            .into()
        } else {
            text(&msg.content).size(13).color(ThemeColors::TEXT_PRIMARY).into()
        };

        let mut has_actions = false;
        let mut actions = row![].spacing(4);
        if is_assistant && !msg.content.is_empty() && !msg.is_streaming {
            has_actions = true;
            actions = actions.push(
                button(row![lucide::copy().size(11), text("Copy").size(10)].spacing(4))
                    .style(ghost_button_style())
                    .on_press(Message::CopyMessage(idx)),
            );
            if self.has_applyable_request(&msg.content) {
                actions = actions.push(
                    button(
                        row![lucide::arrow_right().size(11), text("Apply").size(10)].spacing(4),
                    )
                    .style(accent_button_style())
                    .on_press(Message::ApplyToRequest(msg.content.clone())),
                );
            }
        }

        let mut bubble_content = column![header, content_text].spacing(6);

        if has_actions {
            bubble_content = bubble_content.push(actions);
        }

        let bubble_bg = if is_user {
            Color::from_rgb(0.15, 0.25, 0.45)
        } else {
            Color::from_rgb(0.16, 0.16, 0.20)
        };

        container(bubble_content.padding(12).max_width(if is_user {
            500.0
        } else {
            700.0
        }))
        .style(move |_theme: &Theme| container::Style {
            background: Some(iced::Background::Color(bubble_bg)),
            border: iced::Border::default().rounded(8),
            ..Default::default()
        })
        .into()
    }

    fn has_applyable_request(&self, content: &str) -> bool {
        let upper = content.to_uppercase();
        (upper.contains("GET ") || upper.contains("POST ") || upper.contains("PUT ")
            || upper.contains("PATCH ") || upper.contains("DELETE "))
            && (upper.contains("HTTP/") || upper.contains("HTTPS://")
                || upper.contains("CONTENT-TYPE"))
    }

    fn render_quick_actions(&self) -> Element<'_, Message, Theme, Renderer> {
        if self.is_streaming {
            return row![].into();
        }

        let actions = vec![
            QuickAction::GenerateRequest,
            QuickAction::ExplainResponse,
            QuickAction::GenerateScript,
            QuickAction::GenerateMockData,
            QuickAction::DebugError,
        ];

        let mut action_row = row![].spacing(6).padding(iced::Padding::from([6, 12]));

        for action in actions {
            let icon = action.icon();
            let label = action.label().to_string();
            let msg = Message::QuickAction(action);
            let btn = button(
                row![icon, text(label).size(11)]
                    .spacing(4)
                    .align_y(Alignment::Center),
            )
            .style(small_button_style())
            .on_press(msg);
            action_row = action_row.push(btn);
        }

        action_row.into()
    }

    fn render_input(&self) -> Element<'_, Message, Theme, Renderer> {
        let placeholder = if self.is_streaming {
            "AI is thinking..."
        } else {
            "Ask AI anything about APIs..."
        };

        let input = text_input(placeholder, &self.input)
            .on_input(Message::InputChanged)
            .on_submit(Message::SendMessage)
            .size(13)
            .padding(iced::Padding::from([10, 12]));

        let send_btn = if self.is_streaming || self.input.trim().is_empty() {
            button(row![lucide::send().size(14)].spacing(4))
                .style(disabled_button_style())
        } else {
            button(row![lucide::send().size(14)].spacing(4))
                .style(accent_button_style())
                .on_press(Message::SendMessage)
        };

        row![input, send_btn]
            .spacing(8)
            .align_y(Alignment::Center)
            .padding(iced::Padding::from([8, 12]))
            .into()
    }

    fn render_settings(&self) -> Element<'_, Message, Theme, Renderer> {
        let back_btn = button(
            row![lucide::arrow_left().size(14), text("Back to Chat")].spacing(4),
        )
        .style(ghost_button_style())
        .on_press(Message::ToggleSettings);

        let title = text("AI Provider Settings").size(16);

        let mut providers_list = column![].spacing(8);

        for (idx, config) in self.providers.iter().enumerate() {
            let default_badge = if config.is_default {
                text(" (default)")
                    .size(10)
                    .color(ThemeColors::SUCCESS)
            } else {
                text("").size(10)
            };

            let provider_row = row![
                column![
                    row![text(&config.name).size(13), default_badge]
                        .spacing(4)
                        .align_y(Alignment::Center),
                    text(format!("{} / {}", config.provider, config.model))
                        .size(11)
                        .color(ThemeColors::TEXT_SECONDARY),
                ]
                .spacing(2),
                iced::widget::Space::new().width(Length::Fill),
                button(text("Set Default").size(11))
                    .style(ghost_button_style())
                    .on_press(Message::SetDefaultProvider(idx)),
                button(text("Delete").size(11))
                    .style(danger_button_style())
                    .on_press(Message::DeleteProvider(idx)),
            ]
            .spacing(8)
            .align_y(Alignment::Center)
            .padding(10);

            providers_list = providers_list.push(
                container(provider_row)
                    .style(|_theme: &Theme| container::Style {
                        background: Some(iced::Background::Color(ThemeColors::BG_LIGHT)),
                        border: iced::Border::default()
                            .color(ThemeColors::BORDER)
                            .rounded(6),
                        ..Default::default()
                    })
                    .width(Length::Fill),
            );
        }

        let add_section = column![
            text("Add New Provider").size(14).color(ThemeColors::TEXT_PRIMARY),
            text("Provider").size(12).color(ThemeColors::TEXT_SECONDARY),
            iced::widget::pick_list(
                AiProvider::all().to_vec(),
                Some(self.selected_provider_type.clone()),
                Message::ProviderSelected,
            ),
            text("Name").size(12).color(ThemeColors::TEXT_SECONDARY),
            text_input("My OpenAI", &self.editing_provider_name)
                .on_input(Message::InputChanged)
                .size(13)
                .padding(8),
            text("API Key").size(12).color(ThemeColors::TEXT_SECONDARY),
            text_input("sk-...", &self.editing_api_key)
                .on_input(Message::ApiKeyChanged)
                .size(13)
                .padding(8)
                .secure(true),
            text("Base URL").size(12).color(ThemeColors::TEXT_SECONDARY),
            text_input("https://api.openai.com/v1", &self.editing_base_url)
                .on_input(Message::BaseUrlChanged)
                .size(13)
                .padding(8),
            text("Model").size(12).color(ThemeColors::TEXT_SECONDARY),
            text_input("gpt-4o", &self.editing_model)
                .on_input(Message::ModelChanged)
                .size(13)
                .padding(8),
            button(
                row![lucide::plus().size(14), text("Add Provider")].spacing(4)
            )
            .style(accent_button_style())
            .on_press(Message::SaveProviderConfig),
        ]
        .spacing(6)
        .padding(12);

        column![
            back_btn,
            title,
            rule::horizontal(1),
            providers_list,
            rule::horizontal(1),
            add_section,
        ]
        .spacing(12)
        .padding(16)
        .into()
    }

    fn render_mock_data(&self) -> Element<'_, Message, Theme, Renderer> {
        let description_input = text_input(
            "Describe the data shape... (e.g. 'user profiles with name, email, avatar, created_at')",
            &self.mock_data.description,
        )
        .on_input(|s| {
            let mut config = self.mock_data.clone();
            config.description = s;
            Message::TabChanged(AiTab::MockData) // placeholder
        })
        .size(13)
        .padding(10)
        .width(Length::Fill);

        let count_input = text_input("Count", &self.mock_data.count)
            .size(13)
            .padding(8)
            .width(Length::Fixed(80.0));

        let format_picker = iced::widget::pick_list(
            MockFormat::all().to_vec(),
            Some(self.mock_data.format),
            |f| Message::TabChanged(AiTab::MockData), // placeholder
        );

        let generate_btn = if self.mock_data.is_generating || self.mock_data.description.is_empty() {
            button(row![lucide::loader().size(14), text("Generate")].spacing(4))
                .style(disabled_button_style())
        } else {
            button(row![lucide::sparkles().size(14), text("Generate Mock Data")].spacing(4))
                .style(accent_button_style())
                .on_press(Message::QuickAction(QuickAction::GenerateMockData))
        };

        let config_row = row![count_input, format_picker, generate_btn]
            .spacing(8)
            .align_y(Alignment::Center);

        let output: Element<'_, Message, Theme, Renderer> = if self.mock_data.output.is_empty() {
            container(
                column![
                    lucide::database::<Theme, Renderer>().size(32),
                    text("Generated mock data will appear here").size(13),
                ]
                .spacing(8)
                .align_x(Alignment::Center),
            )
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into()
        } else {
            scrollable(
                column![
                    text(&self.mock_data.output)
                        .size(12)
                        .color(ThemeColors::TEXT_PRIMARY)
                ]
                .padding(12)
            )
            .height(Length::Fill)
            .into()
        };

        column![
            column![
                text("Mock Data Generator").size(14),
                text("Describe your data and AI will generate realistic mock data for testing")
                    .size(12)
                    .color(ThemeColors::TEXT_SECONDARY),
            ]
            .spacing(4),
            description_input,
            config_row,
            rule::horizontal(1),
            output,
        ]
        .spacing(10)
        .padding(16)
        .height(Length::Fill)
        .into()
    }
}

fn tip_row<'a>(
    icon: Element<'a, Message, Theme, Renderer>,
    label: &'a str,
) -> Element<'a, Message, Theme, Renderer> {
    row![icon, text(label).size(12).color(ThemeColors::TEXT_SECONDARY)]
        .spacing(8)
        .align_y(Alignment::Center)
        .into()
}

fn primary_button_style<'a>() -> button::StyleFn<'a, Theme> {
    Box::new(|_theme: &Theme, _status: button::Status| button::Style {
        background: Some(iced::Background::Color(ThemeColors::ACCENT)),
        text_color: Color::WHITE,
        border: iced::Border::default().rounded(6),
        ..Default::default()
    })
}

fn secondary_button_style<'a>() -> button::StyleFn<'a, Theme> {
    Box::new(|_theme: &Theme, _status: button::Status| button::Style {
        background: Some(iced::Background::Color(ThemeColors::BG_LIGHT)),
        text_color: ThemeColors::TEXT_SECONDARY,
        border: iced::Border::default().rounded(6),
        ..Default::default()
    })
}

fn ghost_button_style<'a>() -> button::StyleFn<'a, Theme> {
    Box::new(|_theme: &Theme, _status: button::Status| button::Style {
        background: None,
        text_color: ThemeColors::TEXT_SECONDARY,
        border: iced::Border::default(),
        ..Default::default()
    })
}

fn accent_button_style<'a>() -> button::StyleFn<'a, Theme> {
    Box::new(|_theme: &Theme, _status: button::Status| button::Style {
        background: Some(iced::Background::Color(ThemeColors::ACCENT)),
        text_color: Color::WHITE,
        border: iced::Border::default().rounded(6),
        ..Default::default()
    })
}

fn small_button_style<'a>() -> button::StyleFn<'a, Theme> {
    Box::new(|_theme: &Theme, _status: button::Status| button::Style {
        background: Some(iced::Background::Color(ThemeColors::BG_HOVER)),
        text_color: ThemeColors::TEXT_SECONDARY,
        border: iced::Border::default().rounded(4),
        ..Default::default()
    })
}

fn disabled_button_style<'a>() -> button::StyleFn<'a, Theme> {
    Box::new(|_theme: &Theme, _status: button::Status| button::Style {
        background: Some(iced::Background::Color(ThemeColors::BG_MEDIUM)),
        text_color: ThemeColors::TEXT_DIM,
        border: iced::Border::default().rounded(6),
        ..Default::default()
    })
}

fn danger_button_style<'a>() -> button::StyleFn<'a, Theme> {
    Box::new(|_theme: &Theme, _status: button::Status| button::Style {
        background: Some(iced::Background::Color(ThemeColors::ERROR_DIM)),
        text_color: ThemeColors::ERROR,
        border: iced::Border::default().rounded(6),
        ..Default::default()
    })
}
