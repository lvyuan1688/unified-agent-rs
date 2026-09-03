# Context Window Manager

unified-agent-rs enforces a token budget per session so conversations stay within the model's
context window without silent truncation.

## Token counting

Tokens are counted per-turn using the provider's tokenizer (tiktoken for OpenAI, custom for
Anthropic). Counts are cached on each message so re-counting after compaction is O(1).

## Budget tiers

| Tier | Max tokens | Compaction aggressiveness |
|------|-----------|--------------------------|
| Conservative | 32k | Stub all tool results older than 5 turns |
| Default | 128k | Stub tool results > 20 turns, summarize > 50 turns |
| Generous | 1M | Minimal compaction (for long-context models) |

## Compaction strategy

When the budget is exceeded:

1. **Stub tool results** — replace full output with `{hash, summary, line_count}`.
2. **Summarize old turns** — group 5 oldest turns, LLM-summarize into one paragraph.
3. **Drop dead turns** — remove turns that were superseded (e.g. a plan that was replaced).
4. **Preserve system prompt** — never compact system prompt, project instructions, or skills.

Each compaction pass is logged as a `CompactionEvent` so the user can audit what was shortened.

## Overflow prevention

Before each LLM call, the manager estimates total tokens. If within 10% of budget, it
preemptively compacts to avoid a mid-conversation hard stop.
