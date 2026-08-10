use super::types::{AiChatRequest, AiChatResponse, AiProviderError};
use async_trait::async_trait;

#[async_trait]
pub trait AiProviderAdapter: Send + Sync {
    async fn chat(&self, request: AiChatRequest) -> Result<AiChatResponse, AiProviderError>;

    async fn chat_stream(
        &self,
        request: AiChatRequest,
    ) -> Result<tokio::sync::mpsc::Receiver<String>, AiProviderError>;

    fn name(&self) -> &str;

    fn available_models(&self) -> Vec<&str>;
}
