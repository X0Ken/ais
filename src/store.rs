use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fs, path::Path};

use crate::claude::ClaudeProfile;
use crate::codex::CodexProfile;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Store {
    pub(crate) version: u32,
    #[serde(default)]
    pub(crate) claude: BTreeMap<String, ClaudeProfile>,
    #[serde(default)]
    pub(crate) codex: BTreeMap<String, CodexProfile>,
}

impl Default for Store {
    fn default() -> Self {
        Self {
            version: 1,
            claude: BTreeMap::new(),
            codex: BTreeMap::new(),
        }
    }
}

pub(crate) fn load_store(path: &Path) -> Result<Store> {
    if !path.exists() {
        return Ok(Store::default());
    }

    let contents = fs::read_to_string(path)
        .with_context(|| format!("failed to read store {}", path.display()))?;
    serde_json::from_str(&contents)
        .with_context(|| format!("failed to parse store {}", path.display()))
}

pub(crate) fn save_store(path: &Path, store: &Store) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create store directory {}", parent.display()))?;
    }

    let contents = serde_json::to_string_pretty(store)?;
    fs::write(path, format!("{contents}\n"))
        .with_context(|| format!("failed to write store {}", path.display()))
}
