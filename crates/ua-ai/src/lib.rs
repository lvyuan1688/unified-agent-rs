//! ua-ai: unified multi-provider LLM API.
//! Exposes a single `LlmProvider` trait and 5 skeleton implementations
//! (OpenAI / Anthropic / Gemini / Ollama / vLLM). The skeleton returns
//! canned responses so the agent loop can be exercised offline.

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionResponse {
    pub text: String,
    pub finish_reason: Option<String>,
    pub usage: Option<Usage>,
}

/// The trait every provider implements.
#[async_trait]
pub trait LlmProvider: Send + Sync {
    fn name(&self) -> &str;
    async fn complete(&self, req: &CompletionRequest) -> Result<CompletionResponse>;
    fn supports_tools(&self) -> bool { false }
    fn supports_streaming(&self) -> bool { false }
}

// ---- 5 skeleton implementations -------------------------------------------

pub struct OpenAiProvider { pub api_key: String }
pub struct AnthropicProvider { pub api_key: String }
pub struct GeminiProvider { pub api_key: String }
pub struct OllamaProvider { pub endpoint: String }
pub struct VllmProvider { pub endpoint: String, pub api_key: Option<String> }

macro_rules! stub_provider {
    ($ty:ty, $name:expr) => {
        #[async_trait]
        impl LlmProvider for $ty {
            fn name(&self) -> &str { $name }
            async fn complete(&self, req: &CompletionRequest) -> Result<CompletionResponse> {
                let last = req.messages.last().cloned().unwrap_or(ChatMessage {
                    role: "user".into(),
                    content: String::new(),
                });
                Ok(CompletionResponse {
                    text: format!("[{} stub] {}", $name, last.content),
                    finish_reason: Some("stop".into()),
                    usage: None,
                })
            }
        }
    };
}

stub_provider!(OpenAiProvider, "openai");
stub_provider!(AnthropicProvider, "anthropic");
stub_provider!(GeminiProvider, "gemini");
stub_provider!(OllamaProvider, "ollama");
stub_provider!(VllmProvider, "vllm");

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn openai_stub() {
        let p = OpenAiProvider { api_key: "k".into() };
        let r = p.complete(&CompletionRequest {
            model: "gpt-4o".into(),
            messages: vec![ChatMessage { role: "user".into(), content: "hi".into() }],
            max_tokens: None,
            temperature: None,
        }).await.unwrap();
        assert!(r.text.contains("openai stub"));
    }
}
