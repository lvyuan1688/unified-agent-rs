# Event Bus Design

unified-agent-rs uses an in-process event bus to decouple agent loops from the subsystems that observe them (logging, telemetry, cost tracking, UI).

## Why not channels?

Direct `tokio::sync::mpsc` channels require the sender to know every receiver in advance. When you add a new subscriber (say, a cost-tracker), you have to rewire every producer. An event bus inverts this: producers emit to the bus, subscribers register interest, and the bus handles fan-out.

## Bus topology

```
                 ┌──────────────┐
   agent loop ──▶│              │──▶ logger (structured, async)
                 │  EventBus    │──▶ telemetry (OTLP exporter)
   tool call ───▶│              │──▶ cost tracker
                 │              │──▶ UI bridge (TUI/Web)
                 └──────────────┘
```

The bus is a single `tokio::sync::broadcast` channel with a configurable per-subscriber buffer. Subscribers process events at their own pace; a slow subscriber fills its buffer and silently drops the oldest event (with a counter increment) rather than blocking the producer.

## Event types

All events implement `AgentEvent` and carry a `trace_id` for correlation:

| Event | When emitted | Key payload |
|---|---|---|
| `SessionStarted` | New session begins | session_id, model, prompt_hash |
| `LlmRequest` | Before LLM call | model, input_tokens |
| `LlmResponse` | After LLM call | output_tokens, latency_ms |
| `ToolDispatched` | Tool call starts | tool_name, args_hash |
| `ToolCompleted` | Tool call ends | exit_code, duration_ms |
| `SessionEnded` | Session closes | total_cost_usd |

## Backpressure strategy

The bus is non-blocking by design. If a subscriber's buffer is full:
1. The oldest event is dropped.
2. A `drop_count` counter on that subscriber is incremented.
3. If `drop_count` exceeds a threshold (default: 1000), the subscriber logs a warning and resets the counter.

This guarantees a misbehaving observer cannot stall the agent loop.

## Subscription API

```rust
let mut sub = bus.subscribe(EventFilter::all());

while let Ok(event) = sub.recv().await {
    match event {
        AgentEvent::LlmResponse(r) => cost_tracker.record(&r),
        AgentEvent::ToolCompleted(t) => metrics.inc_tool_duration(&t),
        _ => {}
    }
}
```

`EventFilter` supports topic-based and type-based filtering so a subscriber only receives events it cares about.

See `docs/PLUGINS.md` for how plugins register additional event types without modifying the core bus.
