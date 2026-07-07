use clap::{Parser, Subcommand};

use crate::claude::ClaudeCommand;
use crate::codex::CodexCommand;

#[derive(Parser)]
#[command(
    name = "ais",
    version,
    about = "Switch AI agent authentication profiles"
)]
pub(crate) struct Cli {
    #[command(subcommand)]
    pub(crate) command: Commands,
}

#[derive(Subcommand)]
pub(crate) enum Commands {
    /// Manage Claude Code environment profiles.
    Claude(ClaudeArgs),
    /// Manage Codex authentication profiles.
    Codex(CodexArgs),
}

#[derive(Parser)]
pub(crate) struct ClaudeArgs {
    #[command(subcommand)]
    pub(crate) command: ClaudeCommand,
}

#[derive(Parser)]
pub(crate) struct CodexArgs {
    #[command(subcommand)]
    pub(crate) command: CodexCommand,
}
