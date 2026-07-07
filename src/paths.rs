use anyhow::{anyhow, Result};
use std::{
    env,
    path::{Path, PathBuf},
};

const STORE_ENV: &str = "AIS_STORE";
const CODEX_HOME_ENV: &str = "AIS_CODEX_HOME";
const DEFAULT_STORE_REL: &str = ".config/ais/codex-auth.json";

pub(crate) fn store_path() -> Result<PathBuf> {
    if let Some(path) = env::var_os(STORE_ENV) {
        return Ok(PathBuf::from(path));
    }

    let home = home_dir()?;
    Ok(home.join(Path::new(DEFAULT_STORE_REL)))
}

pub(crate) fn codex_home() -> Result<PathBuf> {
    if let Some(path) = env::var_os(CODEX_HOME_ENV) {
        return Ok(PathBuf::from(path));
    }

    Ok(home_dir()?.join(".codex"))
}

fn home_dir() -> Result<PathBuf> {
    env::var_os("HOME")
        .map(PathBuf::from)
        .filter(|path| !path.as_os_str().is_empty())
        .ok_or_else(|| anyhow!("HOME is not set"))
}
