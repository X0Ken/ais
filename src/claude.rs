mod env;
mod profile;

#[cfg(test)]
mod tests;

use anyhow::{anyhow, bail, Result};
use clap::Subcommand;
use std::path::Path;

use env::{capture_current_profile, print_env_exports};
pub(crate) use profile::ClaudeProfile;
use profile::{create_profile, ModelOptions};

use crate::{
    paths::store_path,
    store::{load_store, save_store},
};

#[derive(Subcommand)]
pub(crate) enum ClaudeCommand {
    /// Create a Claude Code environment profile.
    Create {
        /// Optional profile name. Defaults to a name derived from the base URL.
        #[arg(long, value_name = "NAME")]
        name: Option<String>,
        /// Set ANTHROPIC_MODEL, default opus model, and default sonnet model together.
        #[arg(long, value_name = "MODEL")]
        default_model: Option<String>,
        /// Set ANTHROPIC_MODEL.
        #[arg(long, value_name = "MODEL")]
        model: Option<String>,
        /// Set ANTHROPIC_DEFAULT_OPUS_MODEL.
        #[arg(long, value_name = "MODEL")]
        opus_model: Option<String>,
        /// Set ANTHROPIC_DEFAULT_SONNET_MODEL.
        #[arg(long, value_name = "MODEL")]
        sonnet_model: Option<String>,
        /// Set ANTHROPIC_DEFAULT_HAIKU_MODEL.
        #[arg(long, value_name = "MODEL")]
        haiku_model: Option<String>,
        /// Set CLAUDE_CODE_SUBAGENT_MODEL.
        #[arg(long, value_name = "MODEL")]
        subagent_model: Option<String>,
        /// Set CLAUDE_CODE_EFFORT_LEVEL.
        #[arg(long, value_name = "LEVEL")]
        effort_level: Option<String>,
        /// Anthropic-compatible base URL, such as https://api.example.com/v1.
        base_url: String,
        /// Auth token for the provider.
        auth_token: String,
    },
    /// Delete a saved Claude Code environment profile.
    Delete {
        /// Profile name to delete.
        name: String,
    },
    /// Print shell exports for a saved Claude Code environment profile.
    Env {
        /// Profile name to export.
        name: String,
    },
    /// List saved Claude Code environment profiles.
    List,
    /// Save current Claude Code environment variables as a named profile.
    Save {
        /// Profile name to save.
        name: String,
    },
}

pub(crate) fn run(command: ClaudeCommand) -> Result<()> {
    let store_path = store_path()?;

    match command {
        ClaudeCommand::Create {
            name,
            default_model,
            model,
            opus_model,
            sonnet_model,
            haiku_model,
            subagent_model,
            effort_level,
            base_url,
            auth_token,
        } => {
            let (name, profile) = create_profile(
                name.as_deref(),
                &base_url,
                &auth_token,
                ModelOptions {
                    model,
                    default_model,
                    default_opus_model: opus_model,
                    default_sonnet_model: sonnet_model,
                    default_haiku_model: haiku_model,
                    subagent_model,
                    effort_level,
                },
            )?;
            let mut store = load_store(&store_path)?;
            store.claude.insert(name.clone(), profile);
            save_store(&store_path, &store)?;
            println!("created claude environment profile '{name}'");
            println!("run: eval \"$(ais claude env {name})\"");
        }
        ClaudeCommand::Delete { name } => {
            delete_profile(&store_path, &name)?;
            println!("deleted claude environment profile '{name}'");
        }
        ClaudeCommand::Env { name } => {
            let store = load_store(&store_path)?;
            let profile = store.claude.get(&name).ok_or_else(|| {
                anyhow!(
                    "claude profile '{name}' not found; run `ais claude list` to see saved profiles"
                )
            })?;
            print_env_exports(profile);
        }
        ClaudeCommand::List => {
            let store = load_store(&store_path)?;
            for (name, profile) in store.claude {
                println!("{name}\t{}", profile.base_url());
            }
        }
        ClaudeCommand::Save { name } => {
            let profile = capture_current_profile()?;
            let mut store = load_store(&store_path)?;
            store.claude.insert(name.clone(), profile);
            save_store(&store_path, &store)?;
            println!("saved claude environment profile '{name}'");
            println!("run: eval \"$(ais claude env {name})\"");
        }
    }

    Ok(())
}

fn delete_profile(store_path: &Path, name: &str) -> Result<()> {
    let mut store = load_store(store_path)?;
    if store.claude.remove(name).is_none() {
        bail!("claude profile '{name}' not found; run `ais claude list` to see saved profiles");
    }
    save_store(store_path, &store)
}
