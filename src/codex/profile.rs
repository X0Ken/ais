use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::{collections::BTreeMap, fs, path::Path};

use super::config::{capture_auth_config, update_codex_config, DEFAULT_WIRE_API};
use crate::naming::{normalize_provider_name, provider_name_from_base_url};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct CodexProfile {
    pub(super) auth: JsonValue,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) auth_config: Option<AuthConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(super) struct AuthConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) model_provider: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) preferred_auth_method: Option<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub(super) model_providers: BTreeMap<String, ProviderConfig>,
    #[serde(default, skip_serializing_if = "FeatureConfig::is_empty")]
    pub(super) features: FeatureConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(super) struct ProviderConfig {
    pub(super) name: String,
    pub(super) base_url: String,
    pub(super) wire_api: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) supports_websockets: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) requires_openai_auth: Option<bool>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub(super) struct FeatureConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) responses_websockets_v2: Option<bool>,
}

impl FeatureConfig {
    pub(super) fn is_empty(&self) -> bool {
        self.responses_websockets_v2.is_none()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum AuthKind {
    OpenAiLogin,
    ApiKey,
}

pub(super) fn capture_current_profile(
    auth_path: &Path,
    config_path: &Path,
) -> Result<CodexProfile> {
    let auth_contents = fs::read_to_string(auth_path)
        .with_context(|| format!("failed to read {}", auth_path.display()))?;
    let auth = serde_json::from_str(&auth_contents)
        .with_context(|| format!("failed to parse {}", auth_path.display()))?;
    let auth_config = capture_auth_config(config_path)?;

    Ok(CodexProfile { auth, auth_config })
}

pub(super) fn create_provider_profile(
    name: Option<&str>,
    base_url: &str,
    api_key: &str,
    websocket: bool,
) -> Result<(String, CodexProfile)> {
    let base_url = base_url.trim();
    if base_url.is_empty() {
        bail!("provider base URL cannot be empty");
    }

    let api_key = api_key.trim();
    if api_key.is_empty() {
        bail!("provider API key cannot be empty");
    }

    let provider_name = match name {
        Some(name) => normalize_provider_name(name)?,
        None => provider_name_from_base_url(base_url)?,
    };

    let profile = CodexProfile {
        auth: serde_json::json!({ "OPENAI_API_KEY": api_key }),
        auth_config: Some(AuthConfig {
            model_provider: Some(provider_name.clone()),
            preferred_auth_method: Some("apikey".to_string()),
            model_providers: BTreeMap::from([(
                provider_name.clone(),
                ProviderConfig {
                    name: provider_name.clone(),
                    base_url: base_url.to_string(),
                    wire_api: DEFAULT_WIRE_API.to_string(),
                    supports_websockets: websocket.then_some(true),
                    requires_openai_auth: Some(true),
                },
            )]),
            features: FeatureConfig {
                responses_websockets_v2: websocket.then_some(true),
            },
        }),
    };

    Ok((provider_name, profile))
}

pub(super) fn apply_profile(
    profile: &CodexProfile,
    auth_path: &Path,
    config_path: &Path,
) -> Result<()> {
    if let Some(parent) = auth_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create Codex directory {}", parent.display()))?;
    }

    write_json(auth_path, &profile.auth)?;
    update_codex_config(
        config_path,
        profile.auth_config.as_ref(),
        auth_kind(&profile.auth),
    )?;
    Ok(())
}

pub(super) fn profile_kind(profile: &CodexProfile) -> &'static str {
    match auth_kind(&profile.auth) {
        AuthKind::OpenAiLogin => "openai",
        AuthKind::ApiKey => "apikey",
    }
}

fn write_json(path: &Path, value: &JsonValue) -> Result<()> {
    let contents = serde_json::to_string_pretty(value)?;
    fs::write(path, format!("{contents}\n"))
        .with_context(|| format!("failed to write {}", path.display()))
}

fn auth_kind(auth: &JsonValue) -> AuthKind {
    let has_tokens = auth
        .get("tokens")
        .and_then(JsonValue::as_object)
        .is_some_and(|tokens| !tokens.is_empty());
    if has_tokens {
        return AuthKind::OpenAiLogin;
    }

    match auth.get("auth_mode").and_then(JsonValue::as_str) {
        Some("chatgpt") | Some("oauth") | Some("login") => AuthKind::OpenAiLogin,
        _ => AuthKind::ApiKey,
    }
}
