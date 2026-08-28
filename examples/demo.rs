//! Killer demo: unified-agent-rs boots a stub LLM provider, registers
//! a "greet" tool, and runs the agent loop for one iteration — then
//! publishes a "demo.done" event on the in-memory event bus.
//!
//! Run:  cargo run --example demo
//!
//! What you'll see: the stub provider returns a canned reply, the
//! tool registry is queried, the loop completes in one step, and the
//! event bus dispatches the completion signal to a subscriber.

use anyhow::Result;
use serde_json::json;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use ua_agent_core::{run_loop, Tool, ToolRegistry};
use ua_ai::{
    ChatMessage, CompletionRequest, CompletionResponse, LlmProvider,
};
use ua_event_bus::{DispatchResult, EventBus, Event};

// ─── Stub LLM provider (canned reply, no API key needed) ──────────────
struct StubProvider;

#[async_trait::async_trait]
impl LlmProvider for StubProvider {
    fn name(&self) -> &str { "stub" }
    async fn complete(&self, _req: &CompletionRequest) -> Result<CompletionResponse> {
        Ok(CompletionResponse {
            text: "[stub] Hello from unified-agent-rs!".into(),
            finish_reason: Some("stop".into()),
            usage: None,
        })
    }
}

// ─── A demo tool: "greet" ──────────────────────────────────────────────
struct GreetTool;
static GREET_CALLS: AtomicU32 = AtomicU32::new(0);

#[async_trait::async_trait]
impl Tool for GreetTool {
    fn name(&self) -> &str { "greet" }
    async fn invoke(&self, args: &serde_json::Value) -> Result<serde_json::Value> {
        GREET_CALLS.fetch_add(1, Ordering::SeqCst);
        let name = args.get("name").and_then(|v| v.as_str()).unwrap_or("world");
        Ok(json!({ "greeting": format!("Hello, {}!", name) }))
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("🎭  unified-agent-rs  —  demo run");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    // 1. Build provider + tool registry.
    let provider = StubProvider;
    let mut registry = ToolRegistry::new();
    registry.register(Box::new(GreetTool));
    println!("\n①  Provider: {}  tools registered: greet", provider.name());

    // 2. Invoke the "greet" tool directly to show it works.
    let args = json!({ "name": "Rust" });
    let result = registry.get("greet").unwrap().invoke(&args).await?;
    println!("\n②  Tool 'greet' invoked with {:?}:", args);
    println!("   result: {}", result);

    // 3. Run the agent loop for one iteration.
    let req = CompletionRequest {
        model: "stub".into(),
        messages: vec![ChatMessage {
            role: "user".into(),
            content: "Say hello".into(),
        }],
        max_tokens: Some(64),
        temperature: Some(0.0),
    };
    println!("\n③  Run agent loop (max 1 iteration) ...");
    let t0 = std::time::Instant::now();
    let (steps, resp) = run_loop(&provider, &registry, req, 1).await?;
    println!("   ✅  loop completed in {:?}", t0.elapsed());
    println!("   steps: {}  response: {:?}", steps.len(), resp.text);

    // 4. Publish a "demo.done" event on the event bus.
    let bus = EventBus::new(16);
    let _sub_id = bus.subscribe("demo.done", |_event: &Event| {
        println!("   📨  subscriber received demo.done event");
        Ok(())
    });
    println!("\n④  Publish 'demo.done' event on event bus ...");
    let event = Event::new("demo.done", json!({ "steps": steps.len() }));
    let results: Vec<(usize, DispatchResult)> = bus.publish(&event)
        .into_iter()
        .map(|(id, r)| (id.into(), r))
        .collect();
    println!("   ✅  dispatched to {} subscriber(s)", results.len());

    // 5. Summary.
    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📊  Summary");
    println!("   Provider        : {}", provider.name());
    println!("   Tools registered: 1 (greet)");
    println!("   Loop steps      : {}", steps.len());
    println!("   Greet calls     : {}", GREET_CALLS.load(Ordering::SeqCst));
    println!("   Event bus subs  : 1");
    println!();
    println!("⭐  Star unified-agent-rs for more multi-agent patterns:");
    println!("     https://github.com/lvyuan1688/unified-agent-rs");
    Ok(())
}
