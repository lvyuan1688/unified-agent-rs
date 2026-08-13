# Telemetry Dashboard

Built-in TUI dashboard shows real-time metrics:

```
┌─ Telemetry Dashboard ──────────────────────────┐
│ Provider       Tokens    Latency    Errors     │
│ ─────────────  ───────  ─────────  ────────    │
│ openai         12,450    340ms      0          │
│ anthropic       8,200    520ms      1 (retry)  │
│ ollama          2,100    180ms      0          │
│                                                 │
│ Total cost: $0.043    Session: 00:12:34        │
└─────────────────────────────────────────────────┘
```

## Sinks

- stdout (default)
- JSONL (`--telemetry-sink jsonl:events.jsonl`)
- Prometheus (TODO)
