mod config;
mod profile;

#[cfg(test)]
mod tests;

use anyhow::{anyhow, bail, Context, Result};
use clap::Subcommand;
use std::path::Path;

pub(crate) use profile::CodexProfile;
use profile::{apply_profile, capture_current_profile, create_provider_profile, profile_kind};

use crate::{
    paths::{codex_home, store_path},
    store::{load_store, save_store},
};

#[derive(Subcommand)]
pub(crate) enum CodexCommand {
    /// Create a Codex API key provider profile and switch to it.
    Create {
        /// Optional profile/provider name. Defaults to a name derived from the base URL.
        #[arg(long, value_name = "NAME")]
        name: Option<String>,
        /// Enable Codex responses websocket support for this provider profile.
        #[arg(long = "websocket", visible_alias = "ws")]
        websocket: bool,
        /// Provider base URL, such as https://api.example.com/v1.
        base_url: String,
        /// API key for the provider.
        api_key: String,
    },
    /// Save current Codex authentication as a named profile.
    Save {
        /// Profile name to save.
        name: String,
    },
    /// Switch to a saved Codex authentication profile.
    Switch {
        /// Profile name to switch to.
        name: String,
    },
    /// Delete a saved Codex authentication profile.
    Delete {
        /// Profile name to delete.
        name: String,
    },
    /// List saved Codex authentication profiles.
    List,
}

pub(crate) fn run(command: CodexCommand) -> Result<()> {
    let store_path = store_path()?;
    let codex_home = codex_home()?;
    let codex_auth_path = codex_home.join("auth.json");
    let codex_config_path = codex_home.join("config.toml");

    match command {
        CodexCommand::Create {
            name,
            websocket,
            base_url,
            api_key,
        } => {
            let (name, profile) =
                create_provider_profile(name.as_deref(), &base_url, &api_key, websocket)?;
            let mut store = load_store(&store_path)?;
            store.codex.insert(name.clone(), profile.clone());
            save_store(&store_path, &store)?;
            apply_profile(&profile, &codex_auth_path, &codex_config_path)?;
            println!("created and switched codex provider profile '{name}'");
        }
        CodexCommand::Switch { name } => {
            let store = load_store(&store_path)?;
            let profile = store.codex.get(&name).ok_or_else(|| {
                anyhow!(
                    "codex profile '{name}' not found; run `ais codex list` to see saved profiles"
                )
            })?;
            apply_profile(profile, &codex_auth_path, &codex_config_path)?;
            println!("switched codex authentication to profile '{name}'");
        }
        CodexCommand::Delete { name } => {
            delete_codex_profile(&store_path, &name)?;
            println!("deleted codex authentication profile '{name}'");
        }
        CodexCommand::Save { name } => {
            let profile = capture_current_profile(&codex_auth_path, &codex_config_path)
                .with_context(|| "failed to capture current Codex authentication")?;
            let mut store = load_store(&store_path)?;
            store.codex.insert(name.clone(), profile);
            save_store(&store_path, &store)?;
            println!("saved codex authentication profile '{name}'");
        }
        CodexCommand::List => {
            let store = load_store(&store_path)?;
            for (name, profile) in store.codex {
                println!("{name}\t{}", profile_kind(&profile));
            }
        }
    }

    Ok(())
}

fn delete_codex_profile(store_path: &Path, name: &str) -> Result<()> {
    let mut store = load_store(store_path)?;
    if store.codex.remove(name).is_none() {
        bail!("codex profile '{name}' not found; run `ais codex list` to see saved profiles");
    }
    save_store(store_path, &store)
}
