mod presentation;
mod tui;
use clap::{Parser, Subcommand};
use std::io::{self, Write};

#[derive(Parser)]
#[command(version, about = "Read-only Linux network status (early foundation)")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Observe interface/address state without changing networking
    Status {
        /// Emit the versioned JSON snapshot envelope
        #[arg(long)]
        json: bool,
        /// Use labeled synthetic data; never inspect the host
        #[arg(long)]
        demo: bool,
    },
    /// Browse interface/address state; r refreshes, q quits
    Tui {
        /// Use labeled synthetic data; never inspect the host
        #[arg(long)]
        demo: bool,
    },
}

#[tokio::main]
async fn main() -> std::process::ExitCode {
    match run(Cli::parse()).await {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("nicco: {error}");
            std::process::ExitCode::FAILURE
        }
    }
}

async fn run(cli: Cli) -> Result<(), Box<dyn std::error::Error>> {
    match cli.command {
        Command::Status { json, demo } => {
            let snapshot = nicco_core::collect(demo).await?;
            let output = if json {
                serde_json::to_string_pretty(&snapshot)?
            } else {
                presentation::human(&snapshot)
            };
            match writeln!(io::stdout().lock(), "{output}") {
                Err(e) if e.kind() == io::ErrorKind::BrokenPipe => Ok(()),
                result => Ok(result?),
            }
        }
        Command::Tui { demo } => tui::run(demo).await,
    }
}
