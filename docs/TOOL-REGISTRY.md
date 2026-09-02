# Tool Registry

unified-agent-rs exposes capabilities to the model through a typed tool registry rather than
hard-coded branches. Tools can be added at runtime.

## ToolDef

```rust
pub struct ToolDef {
    pub name: String,            // stable, kebab-case
    pub description: String,     // what the model reads
    pub args_schema: JsonSchema, // used for both validation + prompt
    pub handler: Arc<dyn ToolHandler>,
    pub version: u32,
    pub capabilities: BitFlags,  // which providers may see it
}
```

## Registration

`registry.register(def)` validates the schema up front (draft 2020-12) and rejects duplicate
names unless the version bumps. The model-facing manifest is regenerated lazily so adding a tool
mid-session takes effect on the next turn.

## Exposure gating

A provider that does not support parallel tool calls gets a `max_tool_calls=1` wrapper that
serializes; providers without structured output get args serialized to a strict text block.

## Validation

Args are validated against `args_schema` *before* the handler runs. Rejections return a
structured `ToolError{ field, expected, got }` which is fed back to the model so it can
self-correct instead of the tool failing at runtime.

## Versioning

Handlers are versioned. A schema change that is backwards-incompatible bumps `version` and
the old version stays registered for one deprecation window.
