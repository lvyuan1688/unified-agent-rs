# FS state (v0.1.7)

> `crates/ua-fs-state` — filesystem-backed agent state checkpointing.

## Why

Agents that crash mid-task lose everything. `FsStateStore` persists
per-run state to disk so an agent can resume after a crash.

## Layout

```
<root>/
  <run_id>/
    tasks.json        # the agent's task queue
    messages.jsonl    # every chat message, one JSON per line
    state.json        # user-defined serializable state
```

## API

```rust
use ua_fs_state::FsStateStore;
use serde::{Serialize, Deserialize};

let store = FsStateStore::new("~/.unified-agent-rs/state")?;
let run = store.run("2026-08-20-001")?;

run.save_tasks(&task_queue)?;
run.append_message(&chat_msg)?;
let msgs: Vec<Msg> = run.load_messages()?;
```

## Resuming after crash

```rust
let runs = store.list_runs()?;  // sorted ascending
let run = store.run(&runs.last().unwrap())?;
let tasks: Vec<Task> = run.load_tasks()?;
let msgs: Vec<Msg> = run.load_messages()?;
// ... pick up where we left off ...
```

## Edge cases

- `load_messages` on missing file → `Vec::new()`
- `list_runs` on missing root → `Vec::new()`
- `purge` on missing dir → no-op

## What's NOT in v0.1.7

- Atomic writes (currently `write_all` directly; could use `tempfile` + rename)
- Checksum verification on load
- Compaction (messages.jsonl grows unboundedly)
- Concurrent-run safety (assumes one writer per run)
