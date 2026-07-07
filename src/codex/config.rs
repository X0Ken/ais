use anyhow::{bail, Context, Result};
use std::{collections::BTreeMap, fs, path::Path};
use toml_edit::{value, DocumentMut, Item, Table};

use super::profile::{AuthConfig, AuthKind, FeatureConfig, ProviderConfig};

pub(super) const DEFAULT_WIRE_API: &str = "responses";

pub(super) fn capture_auth_config(config_path: &Path) -> Result<Option<AuthConfig>> {
    if !config_path.exists() {
        return Ok(None);
    }

    let contents = fs::read_to_string(config_path)
        .with_context(|| format!("failed to read {}", config_path.display()))?;
    let doc = contents
        .parse::<DocumentMut>()
        .with_context(|| format!("failed to parse {}", config_path.display()))?;

    let model_provider = doc
        .get("model_provider")
        .and_then(|item| item.as_str())
        .map(ToOwned::to_owned);
    let preferred_auth_method = doc
        .get("preferred_auth_method")
        .and_then(|item| item.as_str())
        .map(ToOwned::to_owned);

    let mut model_providers = BTreeMap::new();
    if let Some(model_provider) = &model_provider {
        if let Some(provider) = doc
            .get("model_providers")
            .and_then(Item::as_table)
            .and_then(|providers| providers.get(model_provider))
            .and_then(Item::as_table)
            .and_then(|table| provider_from_table(model_provider, table))
        {
            model_providers.insert(model_provider.clone(), provider);
        }
    }
    let features = features_from_doc(&doc);

    if model_provider.is_none()
        && preferred_auth_method.is_none()
        && model_providers.is_empty()
        && features.is_empty()
    {
        return Ok(None);
    }

    Ok(Some(AuthConfig {
        model_provider,
        preferred_auth_method,
        model_providers,
        features,
    }))
}

pub(super) fn update_codex_config(
    config_path: &Path,
    auth_config: Option<&AuthConfig>,
    auth_kind: AuthKind,
) -> Result<()> {
    let mut doc = if config_path.exists() {
        let contents = fs::read_to_string(config_path)
            .with_context(|| format!("failed to read {}", config_path.display()))?;
        contents
            .parse::<DocumentMut>()
            .with_context(|| format!("failed to parse {}", config_path.display()))?
    } else {
        DocumentMut::new()
    };

    remove_auth_config(&mut doc);

    match (auth_config, auth_kind) {
        (Some(auth_config), _) => apply_auth_config(&mut doc, auth_config),
        (None, AuthKind::ApiKey) => {
            bail!("API key profile is missing provider configuration")
        }
        (None, AuthKind::OpenAiLogin) => {}
    }

    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create Codex directory {}", parent.display()))?;
    }

    fs::write(config_path, doc.to_string())
        .with_context(|| format!("failed to write {}", config_path.display()))
}

fn provider_from_table(table_name: &str, table: &Table) -> Option<ProviderConfig> {
    let base_url = table.get("base_url")?.as_str()?.to_string();
    Some(ProviderConfig {
        name: table
            .get("name")
            .and_then(|item| item.as_str())
            .unwrap_or(table_name)
            .to_string(),
        base_url,
        wire_api: table
            .get("wire_api")
            .and_then(|item| item.as_str())
            .unwrap_or(DEFAULT_WIRE_API)
            .to_string(),
        supports_websockets: table
            .get("supports_websockets")
            .and_then(|item| item.as_bool()),
        requires_openai_auth: table
            .get("requires_openai_auth")
            .and_then(|item| item.as_bool()),
    })
}

fn features_from_doc(doc: &DocumentMut) -> FeatureConfig {
    let features = doc.get("features").and_then(Item::as_table);
    FeatureConfig {
        responses_websockets_v2: features
            .and_then(|table| table.get("responses_websockets_v2"))
            .and_then(|item| item.as_bool()),
    }
}

fn remove_auth_config(doc: &mut DocumentMut) {
    doc.remove("model_provider");
    doc.remove("preferred_auth_method");
    if let Some(features) = doc.get_mut("features").and_then(Item::as_table_mut) {
        features.remove("responses_websockets_v2");
    }
}

fn apply_auth_config(doc: &mut DocumentMut, auth_config: &AuthConfig) {
    if let Some(model_provider) = &auth_config.model_provider {
        doc["model_provider"] = value(model_provider);
    }
    if let Some(preferred_auth_method) = &auth_config.preferred_auth_method {
        doc["preferred_auth_method"] = value(preferred_auth_method);
    }
    if !auth_config.model_providers.is_empty() {
        if !doc.contains_key("model_providers") {
            let mut table = Table::new();
            table.set_implicit(true);
            doc["model_providers"] = Item::Table(table);
        }
        let Some(providers) = doc["model_providers"].as_table_mut() else {
            let mut table = Table::new();
            table.set_implicit(true);
            doc["model_providers"] = Item::Table(table);
            let Some(providers) = doc["model_providers"].as_table_mut() else {
                return;
            };
            write_model_providers(providers, &auth_config.model_providers);
            return;
        };
        write_model_providers(providers, &auth_config.model_providers);
    }
    apply_feature_config(doc, &auth_config.features);
}

fn write_model_providers(
    providers: &mut Table,
    model_providers: &BTreeMap<String, ProviderConfig>,
) {
    for (name, provider) in model_providers {
        providers[name] = Item::Table(provider_to_table(provider));
    }
}

fn provider_to_table(provider: &ProviderConfig) -> Table {
    let mut table = Table::new();
    table["name"] = value(&provider.name);
    table["base_url"] = value(&provider.base_url);
    table["wire_api"] = value(&provider.wire_api);
    if let Some(supports_websockets) = provider.supports_websockets {
        table["supports_websockets"] = value(supports_websockets);
    }
    if let Some(requires_openai_auth) = provider.requires_openai_auth {
        table["requires_openai_auth"] = value(requires_openai_auth);
    }
    table
}

fn apply_feature_config(doc: &mut DocumentMut, features: &FeatureConfig) {
    if features.is_empty() {
        return;
    }

    if !doc.contains_key("features") || !doc["features"].is_table() {
        doc["features"] = Item::Table(Table::new());
    }

    let Some(features_table) = doc["features"].as_table_mut() else {
        return;
    };
    if let Some(responses_websockets_v2) = features.responses_websockets_v2 {
        features_table["responses_websockets_v2"] = value(responses_websockets_v2);
    }
}
