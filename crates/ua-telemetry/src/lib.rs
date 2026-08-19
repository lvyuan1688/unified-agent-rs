//! ua-telemetry: in-process telemetry for unified-agent-rs.
//! Records per-iteration token usage, tool invocations, and errors.
//! Queryable for live dashboards.

use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use std::collections::VecDeque;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub ts: u64,
    pub kind: String,
    pub data: serde_json::Value,
}

pub struct Telemetry {
    events: Mutex<VecDeque<Event>>,
    max: usize,
}

impl Telemetry {
    pub fn new(max: usize) -> Self {
        Self { events: Mutex::new(VecDeque::with_capacity(max)), max }
    }

    pub fn record(&self, kind: impl Into<String>, data: serde_json::Value) {
        let mut g = self.events.lock().unwrap();
        if g.len() >= self.max { g.pop_front(); }
        g.push_back(Event {
            ts: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
            kind: kind.into(),
            data,
        });
    }

    pub fn snapshot(&self) -> Vec<Event> {
        self.events.lock().unwrap().iter().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_and_snapshot() {
        let t = Telemetry::new(8);
        t.record("tool", serde_json::json!({"name": "read_file"}));
        let s = t.snapshot();
        assert_eq!(s.len(), 1);
        assert_eq!(s[0].kind, "tool");
    }
}
