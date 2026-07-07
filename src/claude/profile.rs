use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

use crate::naming::{normalize_provider_name, provider_name_from_base_url};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct ClaudeProfile {
    base_url: String,
    auth_token: String,
    #[serde(default = "default_disable_nonessential_traffic")]
    disable_nonessential_traffic: bool,
    #[serde(default)]
    attribution_header: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    default_opus_model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    default_sonnet_model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    default_haiku_model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    subagent_model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    effort_level: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct ModelOptions {
    pub(super) model: Option<String>,
    pub(super) default_model: Option<String>,
    pub(super) default_opus_model: Option<String>,
    pub(super) default_sonnet_model: Option<String>,
    pub(super) default_haiku_model: Option<String>,
    pub(super) subagent_model: Option<String>,
    pub(super) effort_level: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct TrafficOptions {
    pub(super) disable_nonessential_traffic: bool,
    pub(super) attribution_header: bool,
}

impl Default for TrafficOptions {
    fn default() -> Self {
        Self {
            disable_nonessential_traffic: true,
            attribution_header: false,
        }
    }
}

impl ClaudeProfile {
    pub(crate) fn base_url(&self) -> &str {
        &self.base_url
    }

    pub(super) fn auth_token(&self) -> &str {
        &self.auth_token
    }

    pub(super) fn disable_nonessential_traffic(&self) -> bool {
        self.disable_nonessential_traffic
    }

    pub(super) fn attribution_header(&self) -> bool {
        self.attribution_header
    }

    pub(super) fn model(&self) -> Option<&str> {
        self.model.as_deref()
    }

    pub(super) fn default_opus_model(&self) -> Option<&str> {
        self.default_opus_model.as_deref()
    }

    pub(super) fn default_sonnet_model(&self) -> Option<&str> {
        self.default_sonnet_model.as_deref()
    }

    pub(super) fn default_haiku_model(&self) -> Option<&str> {
        self.default_haiku_model.as_deref()
    }

    pub(super) fn subagent_model(&self) -> Option<&str> {
        self.subagent_model.as_deref()
    }

    pub(super) fn effort_level(&self) -> Option<&str> {
        self.effort_level.as_deref()
    }
}

pub(super) fn create_profile(
    name: Option<&str>,
    base_url: &str,
    auth_token: &str,
    models: ModelOptions,
) -> Result<(String, ClaudeProfile)> {
    let profile = build_profile(base_url, auth_token, TrafficOptions::default(), models)?;
    let profile_name = match name {
        Some(name) => normalize_provider_name(name)?,
        None => provider_name_from_base_url(profile.base_url())?,
    };

    Ok((profile_name, profile))
}

pub(super) fn build_profile(
    base_url: &str,
    auth_token: &str,
    traffic: TrafficOptions,
    models: ModelOptions,
) -> Result<ClaudeProfile> {
    let base_url = base_url.trim();
    if base_url.is_empty() {
        bail!("provider base URL cannot be empty");
    }

    let auth_token = auth_token.trim();
    if auth_token.is_empty() {
        bail!("provider auth token cannot be empty");
    }

    let model = clean_optional(models.model, "model")?.or_else(|| {
        clean_optional(models.default_model.clone(), "default model")
            .ok()
            .flatten()
    });
    let default_model = clean_optional(models.default_model, "default model")?;
    let default_opus_model =
        clean_optional(models.default_opus_model, "default opus model")?.or(default_model.clone());
    let default_sonnet_model = clean_optional(models.default_sonnet_model, "default sonnet model")?
        .or(default_model.clone());
    let default_haiku_model = clean_optional(models.default_haiku_model, "default haiku model")?;
    let subagent_model = clean_optional(models.subagent_model, "subagent model")?;
    let effort_level = clean_optional(models.effort_level, "effort level")?;

    Ok(ClaudeProfile {
        base_url: base_url.to_string(),
        auth_token: auth_token.to_string(),
        disable_nonessential_traffic: traffic.disable_nonessential_traffic,
        attribution_header: traffic.attribution_header,
        model,
        default_opus_model,
        default_sonnet_model,
        default_haiku_model,
        subagent_model,
        effort_level,
    })
}

fn default_disable_nonessential_traffic() -> bool {
    true
}

fn clean_optional(value: Option<String>, label: &str) -> Result<Option<String>> {
    let Some(value) = value else {
        return Ok(None);
    };
    let value = value.trim();
    if value.is_empty() {
        bail!("{label} cannot be empty");
    }
    Ok(Some(value.to_string()))
}
