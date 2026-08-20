//! ua-fs-state: filesystem-backed agent state checkpointing.
//!
//! Each agent run gets a `RunId`. The state store persists:
//!   - `tasks.json`     — the agent's current task queue
//!   - `messages.jsonl` — every chat message (one JSON per line)
//!   - `state.json`     — user-defined serializable state
//!
//! On crash, `restore(run_id)` reads the files back so the agent can
//! resume. The store is intentionally local-first (one JSON file per
//! run) — no DB, no cloud.

use anyhow::Result;
use serde::{de::DeserializeOwned, Serialize};
use std::path::{Path, PathBuf};

/// One run's checkpoint directory.
pub struct RunState {
    pub run_id: String,
    pub dir: PathBuf,
}

impl RunState {
    pub fn new(root: &Path, run_id: impl Into<String>) -> Result<Self> {
        let run_id = run_id.into();
        let dir = root.join(&run_id);
        std::fs::create_dir_all(&dir)?;
        Ok(Self { run_id, dir })
    }

    /// Save the task queue.
    pub fn save_tasks<T: Serialize>(&self, tasks: &T) -> Result<()> {
        save_json(&self.dir.join("tasks.json"), tasks)
    }

    /// Load the task queue.
    pub fn load_tasks<T: DeserializeOwned>(&self) -> Result<T> {
        load_json(&self.dir.join("tasks.json"))
    }

    /// Append one chat message to the JSONL log.
    pub fn append_message<T: Serialize>(&self, msg: &T) -> Result<()> {
        let path = self.dir.join("messages.jsonl");
        let line = serde_json::to_string(msg)? + "\n";
        std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)?
            .write_all(line.as_bytes())?;
        Ok(())
    }

    /// Read every message from the JSONL log.
    pub fn load_messages<T: DeserializeOwned>(&self) -> Result<Vec<T>> {
        let path = self.dir.join("messages.jsonl");
        if !path.exists() {
            return Ok(Vec::new());
        }
        let mut out = Vec::new();
        for line in std::fs::read_to_string(&path)?.lines() {
            if line.trim().is_empty() { continue; }
            out.push(serde_json::from_str(line)?);
        }
        Ok(out)
    }

    /// Save arbitrary user state.
    pub fn save_state<T: Serialize>(&self, state: &T) -> Result<()> {
        save_json(&self.dir.join("state.json"), state)
    }

    /// Load arbitrary user state.
    pub fn load_state<T: DeserializeOwned>(&self) -> Result<T> {
        load_json(&self.dir.join("state.json"))
    }

    /// Delete the run directory.
    pub fn purge(&self) -> Result<()> {
        if self.dir.exists() {
            std::fs::remove_dir_all(&self.dir)?;
        }
        Ok(())
    }
}

/// Top-level state store. Manages multiple runs.
pub struct FsStateStore {
    pub root: PathBuf,
}

impl FsStateStore {
    pub fn new(root: impl Into<PathBuf>) -> Result<Self> {
        let root = root.into();
        std::fs::create_dir_all(&root)?;
        Ok(Self { root })
    }

    pub fn run(&self, run_id: impl Into<String>) -> Result<RunState> {
        RunState::new(&self.root, run_id)
    }

    pub fn list_runs(&self) -> Result<Vec<String>> {
        if !self.root.exists() {
            return Ok(Vec::new());
        }
        let mut out = Vec::new();
        for e in std::fs::read_dir(&self.root)? {
            let e = e?;
            if e.file_type()?.is_dir() {
                if let Some(n) = e.file_name().to_str() {
                    out.push(n.to_string());
                }
            }
        }
        out.sort();
        Ok(out)
    }
}

// ---- helpers -------------------------------------------------------------

fn save_json<T: Serialize>(path: &Path, v: &T) -> Result<()> {
    if let Some(p) = path.parent() {
        std::fs::create_dir_all(p)?;
    }
    let s = serde_json::to_string_pretty(v)?;
    std::fs::write(path, s)?;
    Ok(())
}

fn load_json<T: DeserializeOwned>(path: &Path) -> Result<T> {
    let s = std::fs::read_to_string(path)?;
    Ok(serde_json::from_str(&s)?)
}

// To compile, add to Cargo.toml:
//   serde = { version = "1", features = ["derive"] }
//   serde_json = "1"
//   anyhow = "1"
//   use std::io::Write;  // import at top

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct Task { id: u32, name: String }

    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct Msg { role: String, content: String }

    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct State { counter: u32 }

    #[test]
    fn round_trip_tasks() {
        let tmp = tempdir();
        let store = FsStateStore::new(&tmp).unwrap();
        let run = store.run("r1").unwrap();
        let tasks = vec![Task { id: 1, name: "x".into() }];
        run.save_tasks(&tasks).unwrap();
        let loaded: Vec<Task> = run.load_tasks().unwrap();
        assert_eq!(loaded, tasks);
    }

    #[test]
    fn append_and_load_messages() {
        let tmp = tempdir();
        let store = FsStateStore::new(&tmp).unwrap();
        let run = store.run("r2").unwrap();
        run.append_message(&Msg { role: "user".into(), content: "hi".into() }).unwrap();
        run.append_message(&Msg { role: "assistant".into(), content: "yo".into() }).unwrap();
        let loaded: Vec<Msg> = run.load_messages().unwrap();
        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded[1].content, "yo");
    }

    #[test]
    fn load_messages_missing_file_returns_empty() {
        let tmp = tempdir();
        let store = FsStateStore::new(&tmp).unwrap();
        let run = store.run("r3").unwrap();
        let loaded: Vec<Msg> = run.load_messages().unwrap();
        assert!(loaded.is_empty());
    }

    #[test]
    fn state_round_trip() {
        let tmp = tempdir();
        let store = FsStateStore::new(&tmp).unwrap();
        let run = store.run("r4").unwrap();
        let s = State { counter: 42 };
        run.save_state(&s).unwrap();
        let loaded: State = run.load_state().unwrap();
        assert_eq!(loaded, s);
    }

    #[test]
    fn list_runs_sorted() {
        let tmp = tempdir();
        let store = FsStateStore::new(&tmp).unwrap();
        let _ = store.run("b").unwrap();
        let _ = store.run("a").unwrap();
        let runs = store.list_runs().unwrap();
        assert_eq!(runs, vec!["a".to_string(), "b".to_string()]);
    }

    #[test]
    fn purge_removes_dir() {
        let tmp = tempdir();
        let store = FsStateStore::new(&tmp).unwrap();
        let run = store.run("r5").unwrap();
        run.save_state(&State { counter: 1 }).unwrap();
        run.purge().unwrap();
        assert!(!run.dir.exists());
    }

    fn tempdir() -> PathBuf {
        let p = std::env::temp_dir().join(format!(
            "ua-fs-state-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
        ));
        std::fs::create_dir_all(&p).unwrap();
        p
    }
}

// Force the unused import to be required
mod _io { use std::io::Write; }
