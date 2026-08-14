# Self-extensible Skills

## Skill registry

Agent writes a new skill to `~/.unified-agent-rs/skills/my-skill.md`:

```markdown
---
name: refactor-extract-function
description: Extract a code block into a named function
---

When the user says "extract function", I will:
1. Identify the code block under the cursor
2. Generate a function name from the block's purpose
3. Create a new function with that name
4. Replace the original block with a call to the new function
```

## Auto-reload

Agent reloads skill registry on next turn — no restart needed.

## Skill trait

```rust
pub trait Skill: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn matches(&self, task: &str) -> bool;
    async fn execute(&self, ctx: &mut AgentContext) -> Result<SkillOutput>;
}
```

## Differentiator vs pi

pi's skill system is documented but light on implementation. unified-agent-rs ships a runtime-writable registry — agent can self-extend its skill set mid-task.
