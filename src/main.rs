//! unified-agent-rs CLI entry point.
//! Skeleton: print the resolved provider, then exit.

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "unified-agent-rs", version, about = "Unified AI agent toolkit")]
struct Cli {
    #[command(subcommand)]
    cmd: Option<Command>,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Print the resolved provider name.
    Info,
    /// Send a single completion request to the stub provider.
    Ask { prompt: String },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.cmd.unwrap_or(Command::Info) {
        Command::Info => println!("provider = stub"),
        Command::Ask { prompt } => println!("[stub] {prompt}"),
    }
    Ok(())
}
