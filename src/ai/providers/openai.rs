use crate::ai::provider::AiProviderAdapter;
use crate::ai::types::{AiChatRequest, AiChatResponse, AiProviderError, AiUsage};
use async_trait::async_trait;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};

pub struct OpenAiProvider {
    pub api_key: String,
    pub base_url: String,
    pub http_client: reqwest::Client,
}

impl OpenAiProvider {
    pub fn new(api_key: String, base_url: String) -> Self {
        Self {
            api_key,
            base_url,
            http_client: reqwest::Client::new(),
        }
    }
}

#[derive(Serialize)]
struct ChatCompletionRequest {
    model: String,
    messages: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    stream: bool,
}

#[derive(Serialize, Deserialize)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct ChatCompletionResponse {
    choices: Vec<ChatChoice>,
    usage: Option<CompletionUsage>,
}

#[derive(Deserialize)]
struct ChatChoice {
    message: ChatMessageResponse,
}

#[derive(Deserialize)]
struct ChatMessageResponse {
    content: Option<String>,
}

#[derive(Deserialize)]
struct CompletionUsage {
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u32,
}

#[derive(Deserialize)]
struct StreamChunk {
    choices: Vec<StreamChoice>,
}

#[derive(Deserialize)]
struct StreamChoice {
    delta: Option<StreamDelta>,
    finish_reason: Option<String>,
}

#[derive(Deserialize)]
struct StreamDelta {
    content: Option<String>,
}

#[derive(Deserialize)]
struct ApiError {
    error: ApiErrorDetail,
}

#[derive(Deserialize)]
struct ApiErrorDetail {
    message: String,
    #[serde(rename = "type")]
    error_type: Option<String>,
    code: Option<String>,
}

#[async_trait]
impl AiProviderAdapter for OpenAiProvider {
    async fn chat(&self, request: AiChatRequest) -> Result<AiChatResponse, AiProviderError> {
        let messages: Vec<ChatMessage> = request
            .messages
            .iter()
            .map(|m| ChatMessage {
                role: m.role.to_string(),
                content: m.content.clone(),
            })
            .collect();

        let body = ChatCompletionRequest {
            model: request.model,
            messages,
            max_tokens: request.max_tokens,
            temperature: request.temperature,
            stream: false,
        };

        let response = self
            .http_client
            .post(format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| AiProviderError {
                message: format!("Request failed: {e}"),
                code: None,
            })?;

        let status = response.status();
        let text = response
            .text()
            .await
            .map_err(|e| AiProviderError {
                message: format!("Failed to read response: {e}"),
                code: None,
            })?;

        if !status.is_success() {
            let api_err: Result<ApiError, _> = serde_json::from_str(&text);
            return Err(AiProviderError {
                message: api_err
                    .as_ref()
                    .map(|e| e.error.message.clone())
                    .unwrap_or_else(|_| text.clone()),
                code: Some(status.as_u16().to_string()),
            });
        }

        let completion: ChatCompletionResponse =
            serde_json::from_str(&text).map_err(|e| AiProviderError {
                message: format!("Failed to parse response: {e}"),
                code: None,
            })?;

        let content = completion
            .choices
            .first()
            .and_then(|c| c.message.content.clone())
            .unwrap_or_default();

        let usage = completion
            .usage
            .map(|u| AiUsage {
                prompt_tokens: u.prompt_tokens,
                completion_tokens: u.completion_tokens,
                total_tokens: u.total_tokens,
            })
            .unwrap_or_default();

        Ok(AiChatResponse {
            content,
            model: body.model,
            usage,
        })
    }

    async fn chat_stream(
        &self,
        request: AiChatRequest,
    ) -> Result<tokio::sync::mpsc::Receiver<String>, AiProviderError> {
        let messages: Vec<ChatMessage> = request
            .messages
            .iter()
            .map(|m| ChatMessage {
                role: m.role.to_string(),
                content: m.content.clone(),
            })
            .collect();

        let body = ChatCompletionRequest {
            model: request.model,
            messages,
            max_tokens: request.max_tokens,
            temperature: request.temperature,
            stream: true,
        };

        let response = self
            .http_client
            .post(format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| AiProviderError {
                message: format!("Request failed: {e}"),
                code: None,
            })?;

        let status = response.status();
        if !status.is_success() {
            let text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            let api_err: Result<ApiError, _> = serde_json::from_str(&text);
            return Err(AiProviderError {
                message: api_err
                    .as_ref()
                    .map(|e| e.error.message.clone())
                    .unwrap_or_else(|_| text.clone()),
                code: Some(status.as_u16().to_string()),
            });
        }

        let (tx, rx) = tokio::sync::mpsc::channel(100);
        let stream = response.bytes_stream();

        tokio::spawn(async move {
            let mut buffer = String::new();
            let mut stream = stream;

            while let Some(chunk_result) = stream.next().await {
                let chunk = match chunk_result {
                    Ok(c) => c,
                    Err(e) => {
                        log::warn!("AI stream chunk error: {e}");
                        break;
                    }
                };

                buffer.push_str(&String::from_utf8_lossy(&chunk));

                while let Some(newline_pos) = buffer.find('\n') {
                    let line = buffer[..newline_pos].trim().to_string();
                    buffer = buffer[newline_pos + 1..].to_string();

                    if line.is_empty() || !line.starts_with("data: ") {
                        continue;
                    }

                    let data = &line[6..];
                    if data == "[DONE]" {
                        return;
                    }

                    if let Ok(chunk) = serde_json::from_str::<StreamChunk>(data) {
                        if let Some(choice) = chunk.choices.first() {
                            if let Some(delta) = &choice.delta {
                                if let Some(content) = &delta.content {
                                    if tx.send(content.clone()).await.is_err() {
                                        return;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        });

        Ok(rx)
    }

    fn name(&self) -> &str {
        "OpenAI"
    }

    fn available_models(&self) -> Vec<&str> {
        vec![
            "gpt-4o",
            "gpt-4o-mini",
            "gpt-4-turbo",
            "gpt-4",
            "gpt-3.5-turbo",
            "o1",
            "o1-mini",
            "o1-pro",
        ]
    }
}
