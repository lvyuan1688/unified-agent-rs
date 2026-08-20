//! ua-skill: a self-extensible skill system for unified-agent-rs.
//!
//! A "skill" is a named, versioned prompt template + optional tool list
//! that the agent can load on demand. Skills are discovered from:
//!
//! 1. Built-in skills bundled with the crate
//! 2. User skills at `~/.unified-agent-rs/skills/<name>.md`
//! 3. Project skills at `.unified-agent-rs/skills/<name>.md`
//!
//! The registry merges all three layers; later layers override earlier
//! ones by name. The skeleton ships one built-in skill (`summarize`)
//! and supports loading the rest from disk.

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// A single skill. The `prompt` is what gets injected into the agent's
/// system message when the skill is activated.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skill {
    /// Lowercase, kebab-case identifier. e.g. `"summarize"`.
    pub name: String,
    /// Human-readable description, used by the agent to decide when to
    /// load the skill.
    pub description: String,
    /// SemVer version string, e.g. `"0.1.0"`.
    pub version: String,
    /// The prompt template. May contain `{{var}}` placeholders.
    pub prompt: String,
    /// Optional list of tool names this skill expects to be available.
    pub tools: Vec<String>,
    /// Optional trigger keywords. If any appear in the user's message,
    /// the agent may auto-load the skill.
    pub triggers: Vec<String>,
}

/// A trait for things that can resolve skills by name. The default
/// implementation is `SkillRegistry`, which merges built-in + user +
/// project skill layers.
#[async_trait]
pub trait SkillResolver: Send + Sync {
    async fn get(&self, name: &str) -> Result<Skill>;
    async fn list(&self) -> Result<Vec<String>>;
}

/// The default registry. Looks up skills in this order:
///   1. `project` (`.unified-agent-rs/skills/<name>.md` in `cwd`)
///   2. `user` (`~/.unified-agent-rs/skills/<name>.md`)
///   3. `builtin` (hardcoded `Skill` values)
/// Later layers override earlier ones by name.
pub struct SkillRegistry {
    project_dir: PathBuf,
    user_dir: PathBuf,
    builtin: BTreeMap<String, Skill>,
}

impl SkillRegistry {
    /// Create a registry with default search paths and one built-in
    /// skill (`summarize`).
    pub fn new() -> Self {
        let mut builtin: BTreeMap<String, Skill> = BTreeMap::new();
        builtin.insert(
            "summarize".into(),
            Skill {
                name: "summarize".into(),
                description: "Summarize a long text into N bullet points.".into(),
                version: "0.1.0".into(),
                prompt: "Summarize the following text in at most {{n}} bullet points:\n\n{{text}}".into(),
                tools: vec![],
                triggers: vec!["summary".into(), "summarize".into(), "tl;dr".into()],
            },
        );
        Self {
            project_dir: PathBuf::from(".unified-agent-rs/skills"),
            user_dir: home_skills_dir(),
            builtin,
        }
    }

    /// Override the project skills directory. Useful for tests.
    pub fn project_dir(mut self, p: impl Into<PathBuf>) -> Self {
        self.project_dir = p.into();
        self
    }

    /// Override the user skills directory. Useful for tests.
    pub fn user_dir(mut self, p: impl Into<PathBuf>) -> Self {
        self.user_dir = p.into();
        self
    }

    /// Register a built-in skill. Skills registered here are overridden
    /// by user/project skills of the same name.
    pub fn register_builtin(&mut self, skill: Skill) {
        self.builtin.insert(skill.name.clone(), skill);
    }

    /// Render a skill's `prompt` template with the supplied variables.
    /// `{{var}}` is replaced by `vars["var"]`. Missing variables are
    /// replaced with the empty string.
    pub fn render(skill: &Skill, vars: &BTreeMap<String, String>) -> String {
        let mut out = skill.prompt.clone();
        for (k, v) in vars {
            let placeholder = format!("{{{{{}}}}}", k);
            out = out.replace(&placeholder, v);
        }
        out
    }

    /// Try to load a skill from disk. Returns `Ok(None)` if the file
    /// doesn't exist.
    fn load_from_disk(dir: &Path, name: &str) -> Result<Option<Skill>> {
        let path = dir.join(format!("{}.md", name));
        if !path.exists() {
            return Ok(None);
        }
        let raw = std::fs::read_to_string(&path)?;
        Ok(Some(parse_skill_md(&raw, name)?))
    }
}

impl Default for SkillRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SkillResolver for SkillRegistry {
    async fn get(&self, name: &str) -> Result<Skill> {
        // 1. project layer
        if let Some(s) = Self::load_from_disk(&self.project_dir, name)? {
            return Ok(s);
        }
        // 2. user layer
        if let Some(s) = Self::load_from_disk(&self.user_dir, name)? {
            return Ok(s);
        }
        // 3. builtin layer
        if let Some(s) = self.builtin.get(name) {
            return Ok(s.clone());
        }
        Err(anyhow!("skill not found: {name}"))
    }

    async fn list(&self) -> Result<Vec<String>> {
        let mut names: std::collections::BTreeSet<String> =
            self.builtin.keys().cloned().collect();
        for dir in [&self.user_dir, &self.project_dir] {
            if dir.exists() {
                for ent in std::fs::read_dir(dir)? {
                    let p = ent?.path();
                    if p.extension().and_then(|e| e.to_str()) == Some("md") {
                        if let Some(stem) = p.file_stem().and_then(|s| s.to_str()) {
                            names.insert(stem.to_string());
                        }
                    }
                }
            }
        }
        Ok(names.into_iter().collect())
    }
}

/// Parse a `.md` skill file. Format:
///
/// ```md
/// ---
/// name: summarize
/// description: Summarize a long text into N bullet points.
/// version: 0.1.0
/// tools: []
/// triggers: ["summary", "summarize", "tl;dr"]
/// ---
///
/// Summarize the following text in at most {{n}} bullet points:
///
/// {{text}}
/// ```
///
/// The YAML front matter is the skill metadata; the body is the `prompt`.
fn parse_skill_md(raw: &str, fallback_name: &str) -> Result<Skill> {
    let body = if let Some(rest) = raw.strip_prefix("---\n") {
        rest.splitn(2, "\n---\n").nth(1).unwrap_or("")
    } else {
        raw
    };
    let front_matter = raw
        .strip_prefix("---\n")
        .and_then(|r| r.split("\n---\n").next())
        .unwrap_or("");

    // Tiny YAML parser: only `key: value` and `key: [list]`.
    let mut name = fallback_name.to_string();
    let mut description = String::new();
    let mut version = "0.1.0".to_string();
    let mut tools: Vec<String> = Vec::new();
    let mut triggers: Vec<String> = Vec::new();
    for line in front_matter.lines() {
        let (k, v) = match line.split_once(':') {
            Some(pair) => pair,
            None => continue,
        };
        let k = k.trim();
        let v = v.trim();
        match k {
            "name" => name = v.trim_matches('"').to_string(),
            "description" => description = v.trim_matches('"').to_string(),
            "version" => version = v.trim_matches('"').to_string(),
            "tools" => tools = parse_list(v),
            "triggers" => triggers = parse_list(v),
            _ => {}
        }
    }
    Ok(Skill {
        name,
        description,
        version,
        prompt: body.trim().to_string(),
        tools,
        triggers,
    })
}

fn parse_list(s: &str) -> Vec<String> {
    let s = s.trim();
    if s == "[]" {
        return Vec::new();
    }
    if let Some(inner) = s.strip_prefix('[').and_then(|x| x.strip_suffix(']')) {
        return inner
            .split(',')
            .map(|x| x.trim().trim_matches('"').to_string())
            .filter(|x| !x.is_empty())
            .collect();
    }
    Vec::new()
}

fn home_skills_dir() -> PathBuf {
    if let Some(home) = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")) {
        PathBuf::from(home).join(".unified-agent-rs/skills")
    } else {
        PathBuf::from(".unified-agent-rs/skills")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn builtin_summarize_resolves() {
        let r = SkillRegistry::new();
        let s = r.get("summarize").await.unwrap();
        assert_eq!(s.name, "summarize");
        assert!(s.prompt.contains("{{n}}"));
    }

    #[tokio::test]
    async fn unknown_skill_errors() {
        let r = SkillRegistry::new();
        assert!(r.get("does-not-exist").await.is_err());
    }

    #[tokio::test]
    async fn list_includes_builtin() {
        let r = SkillRegistry::new();
        let names = r.list().await.unwrap();
        assert!(names.iter().any(|n| n == "summarize"));
    }

    #[test]
    fn render_replaces_placeholders() {
        let skill = Skill {
            name: "x".into(),
            description: "".into(),
            version: "0.1.0".into(),
            prompt: "Hello {{who}}!".into(),
            tools: vec![],
            triggers: vec![],
        };
        let mut vars = BTreeMap::new();
        vars.insert("who".into(), "world".into());
        assert_eq!(SkillRegistry::render(&skill, &vars), "Hello world!");
    }

    #[test]
    fn parse_skill_md_basic() {
        let raw = "---\nname: foo\ndescription: A foo skill.\nversion: 1.2.3\ntools: []\ntriggers: [\"foo\", \"foobar\"]\n---\n\nDo the foo thing.\n";
        let s = parse_skill_md(raw, "fallback").unwrap();
        assert_eq!(s.name, "foo");
        assert_eq!(s.version, "1.2.3");
        assert_eq!(s.triggers, vec!["foo".to_string(), "foobar".to_string()]);
        assert!(s.prompt.contains("Do the foo thing"));
    }
}
