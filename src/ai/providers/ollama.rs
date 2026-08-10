use crate::ai::provider::AiProviderAdapter;
use crate::ai::types::{AiChatRequest, AiChatResponse, AiProviderError, AiUsage};
use async_trait::async_trait;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};

pub struct OllamaProvider {
    pub base_url: String,
    pub http_client: reqwest::Client,
}

impl OllamaProvider {
    pub fn new(base_url: String) -> Self {
        Self {
            base_url,
            http_client: reqwest::Client::new(),
        }
    }
}

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    options: Option<ChatOptions>,
}

#[derive(Serialize)]
struct ChatOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    num_predict: Option<u32>,
}

#[derive(Serialize, Deserialize)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct ChatResponse {
    message: ChatMessageResponse,
    #[serde(default)]
    eval_count: Option<u32>,
    #[serde(default)]
    prompt_eval_count: Option<u32>,
}

#[derive(Deserialize)]
struct ChatMessageResponse {
    content: String,
}

#[derive(Deserialize)]
struct StreamResponse {
    message: Option<StreamDelta>,
    done: bool,
    #[serde(default)]
    eval_count: Option<u32>,
    #[serde(default)]
    prompt_eval_count: Option<u32>,
}

#[derive(Deserialize)]
struct StreamDelta {
    content: String,
}

#[derive(Deserialize)]
struct OllamaError {
    error: String,
}

#[async_trait]
impl AiProviderAdapter for OllamaProvider {
    async fn chat(&self, request: AiChatRequest) -> Result<AiChatResponse, AiProviderError> {
        let messages: Vec<ChatMessage> = request
            .messages
            .iter()
            .map(|m| ChatMessage {
                role: m.role.to_string(),
                content: m.content.clone(),
            })
            .collect();

        let body = ChatRequest {
            model: request.model,
            messages,
            stream: false,
            options: Some(ChatOptions {
                temperature: request.temperature,
                num_predict: request.max_tokens,
            }),
        };

        let response = self
            .http_client
            .post(format!("{}/api/chat", self.base_url))
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
            let api_err: Result<OllamaError, _> = serde_json::from_str(&text);
            return Err(AiProviderError {
                message: api_err
                    .as_ref()
                    .map(|e| e.error.clone())
                    .unwrap_or_else(|_| text.clone()),
                code: Some(status.as_u16().to_string()),
            });
        }

        let chat_response: ChatResponse =
            serde_json::from_str(&text).map_err(|e| AiProviderError {
                message: format!("Failed to parse response: {e}"),
                code: None,
            })?;

        let usage = AiUsage {
            prompt_tokens: chat_response.prompt_eval_count.unwrap_or(0),
            completion_tokens: chat_response.eval_count.unwrap_or(0),
            total_tokens: chat_response
                .prompt_eval_count
                .unwrap_or(0)
                .saturating_add(chat_response.eval_count.unwrap_or(0)),
        };

        Ok(AiChatResponse {
            content: chat_response.message.content,
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

        let body = ChatRequest {
            model: request.model,
            messages,
            stream: true,
            options: Some(ChatOptions {
                temperature: request.temperature,
                num_predict: request.max_tokens,
            }),
        };

        let response = self
            .http_client
            .post(format!("{}/api/chat", self.base_url))
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
            let api_err: Result<OllamaError, _> = serde_json::from_str(&text);
            return Err(AiProviderError {
                message: api_err
                    .as_ref()
                    .map(|e| e.error.clone())
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
                        log::warn!("Ollama stream chunk error: {e}");
                        break;
                    }
                };

                buffer.push_str(&String::from_utf8_lossy(&chunk));

                while let Some(newline_pos) = buffer.find('\n') {
                    let line = buffer[..newline_pos].trim().to_string();
                    buffer = buffer[newline_pos + 1..].to_string();

                    if line.is_empty() {
                        continue;
                    }

                    if let Ok(chunk) = serde_json::from_str::<StreamResponse>(&line) {
                        if chunk.done {
                            return;
                        }
                        if let Some(delta) = chunk.message {
                            if tx.send(delta.content).await.is_err() {
                                return;
                            }
                        }
                    }
                }
            }
        });

        Ok(rx)
    }

    fn name(&self) -> &str {
        "Ollama"
    }

    fn available_models(&self) -> Vec<&str> {
        vec![
            "llama3.1",
            "llama3.1:8b",
            "llama3.1:70b",
            "codellama",
            "mistral",
            "mixtral",
            "phi3",
            "gemma2",
            "qwen2.5",
        ]
    }
}
