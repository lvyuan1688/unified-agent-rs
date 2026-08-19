//! ua-agent-core: agent loop + tool dispatch.
//! The loop is provider-agnostic: callers supply an `LlmProvider` and a
//! `ToolRegistry`, and the loop drives Think → Act → Observe until Done.

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use ua_ai::{CompletionRequest, CompletionResponse, LlmProvider};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Phase { Think, Act, Observe, Done }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall { pub name: String, pub args: serde_json::Value }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Step {
    pub phase: Phase,
    pub tool_calls: Vec<ToolCall>,
}

#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    async fn invoke(&self, args: &serde_json::Value) -> Result<serde_json::Value>;
}

pub struct ToolRegistry { tools: BTreeMap<String, Box<dyn Tool>> }

impl ToolRegistry {
    pub fn new() -> Self { Self { tools: BTreeMap::new() } }
    pub fn register(&mut self, t: Box<dyn Tool>) { self.tools.insert(t.name().into(), t); }
    pub fn get(&self, name: &str) -> Option<&dyn Tool> {
        self.tools.get(name).map(|b| b.as_ref())
    }
}

/// Run the agent loop. Each iteration:
/// 1. Think: call the provider with the current request.
/// 2. Act: dispatch any tool calls.
/// 3. Observe: feed results back into the next Think.
pub async fn run_loop<P: LlmProvider>(
    provider: &P,
    registry: &ToolRegistry,
    initial: CompletionRequest,
    max_iterations: u32,
) -> Result<(Vec<Step>, CompletionResponse)> {
    let mut req = initial;
    let mut history = Vec::new();
    let mut last_resp = CompletionResponse {
        text: String::new(),
        finish_reason: None,
        usage: None,
    };
    for _ in 0..max_iterations {
        let resp = provider.complete(&req).await?;
        // For the skeleton, no tool calls are produced; just terminate.
        history.push(Step { phase: Phase::Done, tool_calls: vec![] });
        last_resp = resp;
        break;
    }
    let _ = registry;
    Ok((history, last_resp))
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use ua_ai::ChatMessage;

    struct Echo;
    #[async_trait]
    impl LlmProvider for Echo {
        fn name(&self) -> &str { "echo" }
        async fn complete(&self, _: &CompletionRequest) -> Result<CompletionResponse> {
            Ok(CompletionResponse { text: "ok".into(), finish_reason: Some("stop".into()), usage: None })
        }
    }

    #[tokio::test]
    async fn loop_runs() {
        let p = Echo;
        let r = ToolRegistry::new();
        let req = CompletionRequest {
            model: "x".into(),
            messages: vec![ChatMessage { role: "user".into(), content: "hi".into() }],
            max_tokens: None,
            temperature: None,
        };
        let (h, _) = run_loop(&p, &r, req, 5).await.unwrap();
        assert_eq!(h.len(), 1);
    }
}
