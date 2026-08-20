# Skills (self-extensible)

> v0.1.5 — `crates/ua-skill` adds a self-extensible skill system.

## What's a skill?

A **skill** is a named, versioned prompt template (plus an optional tool
list and trigger keywords) that the agent can load on demand. Skills
encode reusable behaviors: "summarize this", "write a unit test for
this function", "refactor using extract-method", etc.

## Three layers, last-wins

`SkillRegistry` merges three skill sources:

| Layer | Path | Priority |
|-------|------|----------|
| Project | `.unified-agent-rs/skills/<name>.md` (in `cwd`) | highest |
| User | `~/.unified-agent-rs/skills/<name>.md` | middle |
| Built-in | hard-coded `Skill` values in `lib.rs` | lowest |

A skill named `summarize` in the project layer overrides a user-layer
`summarize`, which overrides the built-in.

## Skill file format

```md
---
name: summarize
description: Summarize a long text into N bullet points.
version: 0.1.0
tools: []
triggers: ["summary", "summarize", "tl;dr"]
---

Summarize the following text in at most {{n}} bullet points:

{{text}}
```

The YAML front matter is the skill metadata; the body is the `prompt`
template. `{{var}}` placeholders are replaced at render time.

## Using skills from the agent loop

```rust
use ua_skill::{SkillRegistry, SkillResolver};

let registry = SkillRegistry::new();
let skill = registry.get("summarize").await?;

let mut vars = BTreeMap::new();
vars.insert("n".into(), "3".into());
vars.insert("text".into(), "long input...".into());

let prompt = SkillRegistry::render(&skill, &vars);
// → "Summarize the following text in at most 3 bullet points:\n\nlong input..."
```

## Built-in skills

v0.1.5 ships one:

| Name | What it does |
|------|--------------|
| `summarize` | Summarize text into `{{n}}` bullet points |

## Adding a skill

1. `mkdir -p ~/.unified-agent-rs/skills`
2. `edit ~/.unified-agent-rs/skills/my-skill.md`
3. The agent picks it up next session.

For project-specific skills (e.g. "how to run this repo's tests"), commit
them to `.unified-agent-rs/skills/` in the project repo.

## Why "self-extensible"?

Skills are just `.md` files. A user (or the agent itself, with write
permission) can drop a new file in the skills directory and it's
immediately discoverable — no recompile, no restart. This mirrors how
Claude Code's `/skills` system and AtomCode's skill catalog work, but
with a much smaller surface area.

## Future work

- Skill provenance: record where each skill was loaded from
- Skill versioning: allow multiple versions side-by-side
- Skill conflicts: surface when project + user skills diverge
- Auto-skill generation: agent writes a skill after solving a problem
