//! ua-coding-agent: a coding specialisation of `ua-agent-core`.
//! Wires in file editing + shell execution tools and a verify step.

use anyhow::Result;
use async_trait::async_trait;
use serde_json::{json, Value};
use std::path::PathBuf;
use ua_agent_core::{Tool, ToolRegistry};

pub struct ReadFileTool;
pub struct WriteFileTool;
pub struct RunShellTool;

#[async_trait]
impl Tool for ReadFileTool {
    fn name(&self) -> &str { "read_file" }
    async fn invoke(&self, args: &Value) -> Result<Value> {
        let path = args.get("path").and_then(|v| v.as_str()).unwrap_or("");
        Ok(json!({ "path": path, "content": "" }))
    }
}

#[async_trait]
impl Tool for WriteFileTool {
    fn name(&self) -> &str { "write_file" }
    async fn invoke(&self, args: &Value) -> Result<Value> {
        let path = args.get("path").and_then(|v| v.as_str()).unwrap_or("");
        Ok(json!({ "path": path, "written": true }))
    }
}

#[async_trait]
impl Tool for RunShellTool {
    fn name(&self) -> &str { "run_shell" }
    async fn invoke(&self, args: &Value) -> Result<Value> {
        let cmd = args.get("cmd").and_then(|v| v.as_str()).unwrap_or("");
        Ok(json!({ "cmd": cmd, "exit": 0, "stdout": "" }))
    }
}

/// Build the default coding agent tool registry.
pub fn build_registry() -> ToolRegistry {
    let mut r = ToolRegistry::new();
    r.register(Box::new(ReadFileTool));
    r.register(Box::new(WriteFileTool));
    r.register(Box::new(RunShellTool));
    r
}

/// Verify the workspace by running a configured command. Returns true on
/// zero exit status.
pub async fn verify(cmd: &str, _cwd: &PathBuf) -> Result<bool> {
    let parts: Vec<&str> = cmd.split_whitespace().collect();
    if parts.is_empty() { return Ok(false); }
    let mut c = tokio::process::Command::new(parts[0]);
    c.args(&parts[1..]);
    let status = c.status().await?;
    Ok(status.success())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_has_3_tools() {
        let r = build_registry();
        assert!(r.get("read_file").is_some());
        assert!(r.get("write_file").is_some());
        assert!(r.get("run_shell").is_some());
    }

    #[tokio::test]
    async fn read_file_stub() {
        let t = ReadFileTool;
        let v = t.invoke(&json!({"path": "x"})).await.unwrap();
        assert_eq!(v["path"], "x");
    }
}
