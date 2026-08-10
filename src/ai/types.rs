use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum AiProvider {
    #[default]
    OpenAi,
    Anthropic,
    Ollama,
    Custom,
}

impl std::fmt::Display for AiProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AiProvider::OpenAi => write!(f, "OpenAI"),
            AiProvider::Anthropic => write!(f, "Anthropic"),
            AiProvider::Ollama => write!(f, "Ollama"),
            AiProvider::Custom => write!(f, "Custom"),
        }
    }
}

impl AiProvider {
    pub fn all() -> &'static [AiProvider] {
        &[
            AiProvider::OpenAi,
            AiProvider::Anthropic,
            AiProvider::Ollama,
            AiProvider::Custom,
        ]
    }

    pub fn default_base_url(&self) -> &str {
        match self {
            AiProvider::OpenAi => "https://api.openai.com/v1",
            AiProvider::Anthropic => "https://api.anthropic.com/v1",
            AiProvider::Ollama => "http://localhost:11434",
            AiProvider::Custom => "",
        }
    }

    pub fn default_model(&self) -> &str {
        match self {
            AiProvider::OpenAi => "gpt-4o",
            AiProvider::Anthropic => "claude-sonnet-4-20250514",
            AiProvider::Ollama => "llama3.1",
            AiProvider::Custom => "",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiProviderConfig {
    pub id: Option<i32>,
    pub provider: AiProvider,
    pub name: String,
    pub base_url: String,
    pub model: String,
    pub max_tokens: u32,
    pub temperature: f32,
    pub is_default: bool,
}

impl Default for AiProviderConfig {
    fn default() -> Self {
        Self {
            id: None,
            provider: AiProvider::OpenAi,
            name: "OpenAI".to_string(),
            base_url: AiProvider::OpenAi.default_base_url().to_string(),
            model: AiProvider::OpenAi.default_model().to_string(),
            max_tokens: 4096,
            temperature: 0.7,
            is_default: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AiRole {
    System,
    User,
    Assistant,
}

impl std::fmt::Display for AiRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AiRole::System => write!(f, "system"),
            AiRole::User => write!(f, "user"),
            AiRole::Assistant => write!(f, "assistant"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiChatMessage {
    pub role: AiRole,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiChatRequest {
    pub model: String,
    pub messages: Vec<AiChatMessage>,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
    pub stream: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiChatResponse {
    pub content: String,
    pub model: String,
    pub usage: AiUsage,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AiUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

#[derive(Debug, Clone)]
pub struct AiProviderError {
    pub message: String,
    pub code: Option<String>,
}

impl std::fmt::Display for AiProviderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.code {
            Some(code) => write!(f, "[{}] {}", code, self.message),
            None => write!(f, "{}", self.message),
        }
    }
}

impl std::error::Error for AiProviderError {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiConversation {
    pub id: Option<i32>,
    pub provider_config_id: i32,
    pub title: Option<String>,
    pub system_prompt: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiMessage {
    pub id: Option<i32>,
    pub conversation_id: i32,
    pub role: AiRole,
    pub content: String,
    pub tokens_used: u32,
    pub created_at: String,
}
