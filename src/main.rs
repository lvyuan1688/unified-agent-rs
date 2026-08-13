//! unified-agent-rs - unified multi-provider LLM API + agent loop + TUI
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "unified-agent-rs", version, about = "Unified agent toolkit")]
struct Cli {
    #[command(subcommand)]
    cmd: Option<Cmd>,
}

#[derive(Subcommand)]
enum Cmd {
    /// Start the TUI
    Tui,
    /// Send a one-shot prompt
    Ask { prompt: String },
    /// List configured LLM providers
    Providers,
    /// Show telemetry dashboard
    Telemetry,
}

fn main() {
    match Cli::parse().cmd.unwrap_or(Cmd::Tui) {
        Cmd::Tui => println!("[tui] launching (stub)"),
        Cmd::Ask { prompt } => println!("[ask] {prompt} (stub)"),
        Cmd::Providers => println!("[providers] openai/anthropic/gemini/ollama (stub)"),
        Cmd::Telemetry => println!("[telemetry] dashboard (stub)"),
    }
}
