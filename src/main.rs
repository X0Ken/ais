mod claude;
mod cli;
mod codex;
mod naming;
mod paths;
mod store;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Commands};

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Claude(args) => claude::run(args.command),
        Commands::Codex(args) => codex::run(args.command),
    }
}
