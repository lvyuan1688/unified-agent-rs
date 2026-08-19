# Contributing to unified-agent-rs

Thanks for your interest! This is a community-driven, open-source AI agent
toolkit. Contributions of all sizes are welcome.

## Quick start

```bash
git clone https://github.com/lvyuan1688/unified-agent-rs
cd unified-agent-rs
cargo build
cargo test
```

The skeleton ships stub providers that return canned responses, so the agent
loop can be exercised without an API key.

## Ways to contribute

- **Bugs**: open an issue with OS, Rust version, command, and stack trace.
- **Providers**: add a new `LlmProvider` in `crates/ua-ai/src/lib.rs`.
- **Tools**: add new `Tool` implementations in
  `crates/ua-coding-agent/src/lib.rs`.
- **Telemetry**: extend `crates/ua-telemetry` with new event kinds.
- **TUI**: improve `crates/ua-tui` rendering and key bindings.
- **Docs**: typos, clarifications, and new guides are all welcome.

## Pull request checklist

- [ ] `cargo fmt` is clean
- [ ] `cargo clippy -- -D warnings` is clean
- [ ] `cargo test` passes
- [ ] `CHANGELOG.md` updated (if user-visible)

## Code of conduct

Be kind. Personal attacks, harassment, or discriminatory behavior will not be
tolerated.

## License

By contributing, you agree your contributions are licensed under the MIT
license (see `LICENSE`).
