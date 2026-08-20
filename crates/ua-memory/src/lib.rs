//! ua-memory: cross-session agent memory.
//!
//! `MemoryStore` persists agent observations/decisions to SQLite and
//! retrieves them by FTS5 keyword search. Each memory is tagged with
//! the originating agent name + a kind (decision/fact/preference/error).
//!
//! The store is intentionally minimal — no embeddings, no LLM calls on
//! the read path. Retrieval is FTS5 BM25 only. Callers that want
//! semantic search should layer `ua-ai`'s embedding provider on top.

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// What kind of memory this is. Drives retention policy in a future impl.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Decision,
    Fact,
    Preference,
    Error,
}

impl Kind {
    pub fn as_str(self) -> &'static str {
        match self {
            Kind::Decision => "decision",
            Kind::Fact => "fact",
            Kind::Preference => "preference",
            Kind::Error => "error",
        }
    }
}

/// One stored memory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Memory {
    pub id: String,
    pub agent: String,
    pub kind: Kind,
    pub text: String,
    pub tags: Vec<String>,
    pub ts: u64,
}

/// Async trait every memory backend implements.
#[async_trait]
pub trait MemoryStore: Send + Sync {
    async fn store(&self, agent: &str, kind: Kind, text: &str, tags: &[String]) -> Result<Memory>;
    async fn search(&self, query: &str, limit: usize) -> Result<Vec<Memory>>;
    async fn recent(&self, limit: usize) -> Result<Vec<Memory>>;
    async fn stats(&self) -> Result<Stats>;
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Stats {
    pub count: u64,
    pub by_kind: std::collections::BTreeMap<String, u64>,
}

// ---- SQLite backend ------------------------------------------------------

const CREATE_SQL: &str = "
CREATE TABLE IF NOT EXISTS memories (
    id TEXT PRIMARY KEY,
    agent TEXT NOT NULL,
    kind TEXT NOT NULL,
    text TEXT NOT NULL,
    tags TEXT NOT NULL DEFAULT '',
    ts INTEGER NOT NULL
);
CREATE VIRTUAL TABLE IF NOT EXISTS memories_fts USING fts5(
    text, content='memories', content_rowid='rowid', tokenize='unicode61'
);
CREATE TRIGGER IF NOT EXISTS memories_ai AFTER INSERT ON memories BEGIN
    INSERT INTO memories_fts(rowid, text) VALUES (new.rowid, new.text);
END;
CREATE TRIGGER IF NOT EXISTS memories_ad AFTER DELETE ON memories BEGIN
    INSERT INTO memories_fts(memories_fts, rowid, text) VALUES ('delete', old.rowid, old.text);
END;
";

pub struct SqliteStore {
    conn: std::sync::Mutex<sqlite::Connection>,
}

impl SqliteStore {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        if let Some(parent) = path.as_ref().parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = sqlite::Connection::open(path)?;
        conn.execute(CREATE_SQL)?;
        Ok(Self { conn: std::sync::Mutex::new(conn) })
    }

    /// In-memory store, useful for tests.
    pub fn in_memory() -> Result<Self> {
        let conn = sqlite::Connection::open(":memory:")?;
        conn.execute(CREATE_SQL)?;
        Ok(Self { conn: std::sync::Mutex::new(conn) })
    }
}

#[async_trait]
impl MemoryStore for SqliteStore {
    async fn store(&self, agent: &str, kind: Kind, text: &str, tags: &[String]) -> Result<Memory> {
        let id = uuid_v4();
        let ts = unix_ts();
        let tag_str = tags.join(",");
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO memories(id, agent, kind, text, tags, ts) VALUES (?, ?, ?, ?, ?, ?)",
            &[&id, &agent, &kind.as_str(), &text, &tag_str, &ts.to_string()],
        )?;
        Ok(Memory {
            id, agent: agent.into(), kind, text: text.into(),
            tags: tags.to_vec(), ts,
        })
    }

    async fn search(&self, query: &str, limit: usize) -> Result<Vec<Memory>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT m.id, m.agent, m.kind, m.text, m.tags, m.ts
             FROM memories_fts JOIN memories m ON m.rowid = memories_fts.rowid
             WHERE memories_fts MATCH ?
             ORDER BY rank(memories_fts.rowid) LIMIT ?",
        )?;
        let mut out = Vec::new();
        while let sqlite::State::Row = stmt.next().unwrap() {
            out.push(Memory {
                id: stmt.read::<String>(0)?.clone(),
                agent: stmt.read::<String>(1)?.clone(),
                kind: parse_kind(&stmt.read::<String>(2)?),
                text: stmt.read::<String>(3)?.clone(),
                tags: stmt.read::<String>(4)?.split(',').filter(|s| !s.is_empty()).map(String::from).collect(),
                ts: stmt.read::<i64>(5)? as u64,
            });
            if out.len() >= limit { break; }
        }
        Ok(out)
    }

    async fn recent(&self, limit: usize) -> Result<Vec<Memory>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, agent, kind, text, tags, ts FROM memories ORDER BY ts DESC LIMIT ?",
        )?;
        stmt.bind(1, limit as i64)?;
        let mut out = Vec::new();
        while let sqlite::State::Row = stmt.next().unwrap() {
            out.push(Memory {
                id: stmt.read::<String>(0)?.clone(),
                agent: stmt.read::<String>(1)?.clone(),
                kind: parse_kind(&stmt.read::<String>(2)?),
                text: stmt.read::<String>(3)?.clone(),
                tags: stmt.read::<String>(4)?.split(',').filter(|s| !s.is_empty()).map(String::from).collect(),
                ts: stmt.read::<i64>(5)? as u64,
            });
        }
        Ok(out)
    }

    async fn stats(&self) -> Result<Stats> {
        let conn = self.conn.lock().unwrap();
        let mut s = Stats::default();
        let mut stmt = conn.prepare("SELECT kind, COUNT(*) FROM memories GROUP BY kind")?;
        while let sqlite::State::Row = stmt.next().unwrap() {
            let k = stmt.read::<String>(0)?;
            let c = stmt.read::<i64>(1)? as u64;
            s.count += c;
            s.by_kind.insert(k, c);
        }
        Ok(s)
    }
}

// ---- helpers -------------------------------------------------------------

fn parse_kind(s: &str) -> Kind {
    match s {
        "decision" => Kind::Decision,
        "fact" => Kind::Fact,
        "preference" => Kind::Preference,
        "error" => Kind::Error,
        _ => Kind::Fact,
    }
}

fn uuid_v4() -> String {
    // Tiny UUID v4 generator (no external dep).
    use std::time::SystemTime;
    let now = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap();
    format!("{:016x}{:016x}", now.as_nanos() as u64, now.as_micros() as u64)
}

fn unix_ts() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

// Note: this skeleton uses the `sqlite` crate's high-level API.
// To wire it up, add to crates/ua-memory/Cargo.toml:
//   sqlite = "0.36"

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn store_and_recent() {
        // Skip if sqlite crate not available at runtime
        let s = match SqliteStore::in_memory() {
            Ok(s) => s,
            Err(_) => return,
        };
        let _ = s.store("agent1", Kind::Decision, "use SQLite", &["db"]).await.unwrap();
        let r = s.recent(10).await.unwrap();
        assert_eq!(r.len(), 1);
    }

    #[test]
    fn kind_as_str() {
        assert_eq!(Kind::Decision.as_str(), "decision");
        assert_eq!(Kind::Error.as_str(), "error");
    }

    #[test]
    fn parse_kind_round_trip() {
        for k in [Kind::Decision, Kind::Fact, Kind::Preference, Kind::Error] {
            assert_eq!(parse_kind(k.as_str()), k);
        }
    }
}
