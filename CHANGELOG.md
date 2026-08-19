# Changelog

All notable changes to unified-agent-rs are documented here. The format
follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the
project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.4] — 2026-08-20

### Added
- `crates/ua-ai` `LlmProvider` trait + 5 stub implementations
  (OpenAI / Anthropic / Gemini / Ollama / vLLM).
- `crates/ua-agent-core` agent loop + `ToolRegistry` + `Tool` trait.
- `crates/ua-coding-agent` file/shell tools + workspace verify.
- `crates/ua-tui` ratatui stepper with `q`/`↑`/`↓` bindings.
- `crates/ua-telemetry` in-process event log with bounded ring buffer.
- `src/main.rs` CLI with `info` and `ask` subcommands.
- `CONTRIBUTING.md`, Issue/PR templates.

## [0.1.3] — 2026-08-15

### Added
- `docs/v0.1.3-patch-notes.md`.

## [0.1.2] — 2026-08-13

### Added
- Initial `LlmProvider` trait draft.

## [0.1.1] — 2026-08-12

### Added
- Stub `OllamaProvider` returning a skeleton response.

## [0.1.0] — 2026-08-10

Initial public skeleton.
