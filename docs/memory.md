# Memory (v0.1.6)

> `crates/ua-memory` — cross-session agent memory.

## Why

Agents that don't remember anything force the user to re-explain context
every session. `ua-memory` persists **decisions, facts, preferences, and
errors** to a local SQLite file and retrieves them with FTS5 keyword
search.

The store is intentionally minimal:
- **No embeddings** — FTS5 BM25 only. Semantic search is a future layer.
- **No LLM calls on read** — retrieval is pure SQL.
- **Local-first** — one SQLite file at `~/.unified-agent-rs/memory.db`.

## The four kinds

| Kind | Example |
|------|---------|
| `Decision` | "we chose SQLite over Postgres for local-first reasons" |
| `Fact` | "the project root is `/code/foo`" |
| `Preference` | "the user prefers tabs over spaces" |
| `Error` | "last run failed with exit code 1 — missing `PGPASSWORD`" |

Future retention policy will drop `Error` memories after N days while
keeping `Decision` / `Fact` / `Preference` indefinitely.

## API

```rust
use ua_memory::{Kind, MemoryStore, SqliteStore};

let store = SqliteStore::open("~/.unified-agent-rs/memory.db")?;

// Store
let m = store.store("coder", Kind::Decision, "use anyhow for errors", &["error-handling"]).await?;

// Retrieve by keyword
let hits = store.search("error handling", 10).await?;

// Recent
let recent = store.recent(10).await?;

// Stats
let stats = store.stats().await?;
// Stats { count: 3, by_kind: {"decision": 1, "fact": 2} }
```

## Trait shape

```rust
#[async_trait]
pub trait MemoryStore: Send + Sync {
    async fn store(&self, agent: &str, kind: Kind, text: &str, tags: &[String]) -> Result<Memory>;
    async fn search(&self, query: &str, limit: usize) -> Result<Vec<Memory>>;
    async fn recent(&self, limit: usize) -> Result<Vec<Memory>>;
    async fn stats(&self) -> Result<Stats>;
}
```

`SqliteStore` is the default impl. A `NoopStore` (for tests) and a
`RemoteStore` (HTTP-backed) are future work.

## What's NOT in v0.1.6

- Embedding-based semantic search
- Retention policy (TTL per kind)
- Memory compaction (merge similar memories)
- Cross-agent memory sharing policy
