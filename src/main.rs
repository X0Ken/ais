use anyhow::{anyhow, bail, Context, Result};
use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::{
    collections::BTreeMap,
    env, fs,
    path::{Path, PathBuf},
};
use toml_edit::{value, DocumentMut, Item, Table};

const STORE_ENV: &str = "AIS_STORE";
const CODEX_HOME_ENV: &str = "AIS_CODEX_HOME";
const DEFAULT_STORE_REL: &str = ".config/ais/codex-auth.json";
const DEFAULT_WIRE_API: &str = "responses";

#[derive(Parser)]
#[command(
    name = "ais",
    version,
    about = "Switch AI agent authentication profiles"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Manage Codex authentication profiles.
    Codex(CodexArgs),
}

#[derive(Parser)]
struct CodexArgs {
    #[command(subcommand)]
    command: CodexCommand,
}

#[derive(Subcommand)]
enum CodexCommand {
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
    /// List saved Codex authentication profiles.
    List,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Store {
    version: u32,
    #[serde(default)]
    codex: BTreeMap<String, CodexProfile>,
}

impl Default for Store {
    fn default() -> Self {
        Self {
            version: 1,
            codex: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CodexProfile {
    auth: JsonValue,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    auth_config: Option<AuthConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct AuthConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    model_provider: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    preferred_auth_method: Option<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    model_providers: BTreeMap<String, ProviderConfig>,
    #[serde(default, skip_serializing_if = "FeatureConfig::is_empty")]
    features: FeatureConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct ProviderConfig {
    name: String,
    base_url: String,
    wire_api: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    supports_websockets: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    requires_openai_auth: Option<bool>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
struct FeatureConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    responses_websockets_v2: Option<bool>,
}

impl FeatureConfig {
    fn is_empty(&self) -> bool {
        self.responses_websockets_v2.is_none()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AuthKind {
    OpenAiLogin,
    ApiKey,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Codex(args) => run_codex(args.command),
    }
}

fn run_codex(command: CodexCommand) -> Result<()> {
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

fn store_path() -> Result<PathBuf> {
    if let Some(path) = env::var_os(STORE_ENV) {
        return Ok(PathBuf::from(path));
    }

    let home = home_dir()?;
    Ok(home.join(DEFAULT_STORE_REL))
}

fn codex_home() -> Result<PathBuf> {
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

fn load_store(path: &Path) -> Result<Store> {
    if !path.exists() {
        return Ok(Store::default());
    }

    let contents = fs::read_to_string(path)
        .with_context(|| format!("failed to read store {}", path.display()))?;
    serde_json::from_str(&contents)
        .with_context(|| format!("failed to parse store {}", path.display()))
}

fn save_store(path: &Path, store: &Store) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create store directory {}", parent.display()))?;
    }

    let contents = serde_json::to_string_pretty(store)?;
    fs::write(path, format!("{contents}\n"))
        .with_context(|| format!("failed to write store {}", path.display()))
}

fn capture_current_profile(auth_path: &Path, config_path: &Path) -> Result<CodexProfile> {
    let auth_contents = fs::read_to_string(auth_path)
        .with_context(|| format!("failed to read {}", auth_path.display()))?;
    let auth = serde_json::from_str(&auth_contents)
        .with_context(|| format!("failed to parse {}", auth_path.display()))?;
    let auth_config = capture_auth_config(config_path)?;

    Ok(CodexProfile { auth, auth_config })
}

fn capture_auth_config(config_path: &Path) -> Result<Option<AuthConfig>> {
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

fn create_provider_profile(
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

fn provider_name_from_base_url(base_url: &str) -> Result<String> {
    let without_scheme = base_url
        .split_once("://")
        .map(|(_, rest)| rest)
        .unwrap_or(base_url);
    let authority = without_scheme
        .split(['/', '?', '#'])
        .next()
        .unwrap_or_default();
    let host_port = authority.rsplit('@').next().unwrap_or(authority);
    let host = if let Some(rest) = host_port.strip_prefix('[') {
        rest.split_once(']')
            .map(|(host, _)| host)
            .unwrap_or(host_port)
    } else {
        host_port.split(':').next().unwrap_or(host_port)
    };

    let mut labels = host.split('.').filter(|label| !label.is_empty());
    let first = labels.next().unwrap_or(host);
    let candidate = if first.eq_ignore_ascii_case("api") {
        labels.next().unwrap_or(first)
    } else {
        first
    };

    normalize_provider_name(candidate)
        .with_context(|| format!("failed to derive provider name from base URL '{base_url}'"))
}

fn normalize_provider_name(name: &str) -> Result<String> {
    let mut normalized = String::new();
    let mut previous_dash = false;

    for ch in name.trim().chars() {
        if ch.is_ascii_alphanumeric() {
            normalized.push(ch.to_ascii_lowercase());
            previous_dash = false;
        } else if !previous_dash {
            normalized.push('-');
            previous_dash = true;
        }
    }

    let normalized = normalized.trim_matches('-').to_string();
    if normalized.is_empty() {
        bail!("provider name cannot be empty");
    }

    Ok(normalized)
}

fn apply_profile(profile: &CodexProfile, auth_path: &Path, config_path: &Path) -> Result<()> {
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

fn write_json(path: &Path, value: &JsonValue) -> Result<()> {
    let contents = serde_json::to_string_pretty(value)?;
    fs::write(path, format!("{contents}\n"))
        .with_context(|| format!("failed to write {}", path.display()))
}

fn update_codex_config(
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
            for (name, provider) in &auth_config.model_providers {
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
                providers[name] = Item::Table(table);
            }
            return;
        };
        for (name, provider) in &auth_config.model_providers {
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
            providers[name] = Item::Table(table);
        }
    }
    apply_feature_config(doc, &auth_config.features);
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

fn profile_kind(profile: &CodexProfile) -> &'static str {
    match auth_kind(&profile.auth) {
        AuthKind::OpenAiLogin => "openai",
        AuthKind::ApiKey => "apikey",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn switch_to_openai_removes_auth_related_config_only() {
        let dir = tempdir().unwrap();
        let auth_path = dir.path().join("auth.json");
        let config_path = dir.path().join("config.toml");
        fs::write(
            &config_path,
            r#"model = "gpt-5.5"
model_provider = "api111"
preferred_auth_method = "apikey"
model_reasoning_effort = "xhigh"

[projects."/root/code/ais"]
trust_level = "trusted"

[model_providers.api111]
name = "api111"
base_url = "https://api.example.com"
wire_api = "responses"
"#,
        )
        .unwrap();

        let profile = CodexProfile {
            auth: serde_json::json!({
                "auth_mode": "chatgpt",
                "OPENAI_API_KEY": null,
                "tokens": { "access_token": "a" },
                "last_refresh": "2026-05-25T00:00:00Z"
            }),
            auth_config: None,
        };

        apply_profile(&profile, &auth_path, &config_path).unwrap();

        let config = fs::read_to_string(config_path).unwrap();
        let doc = config.parse::<DocumentMut>().unwrap();
        assert!(config.contains(r#"model = "gpt-5.5""#));
        assert!(config.contains(r#"model_reasoning_effort = "xhigh""#));
        assert!(config.contains(r#"[projects."/root/code/ais"]"#));
        assert!(doc.get("model_provider").is_none());
        assert!(doc.get("preferred_auth_method").is_none());
        assert!(config.contains(r#"[model_providers.api111]"#));
        assert!(config.contains(r#"base_url = "https://api.example.com""#));
    }

    #[test]
    fn switch_to_apikey_profile_updates_only_auth_related_config() {
        let dir = tempdir().unwrap();
        let auth_path = dir.path().join("auth.json");
        let config_path = dir.path().join("config.toml");
        fs::write(
            &config_path,
            r#"model = "gpt-5.5"
model_reasoning_effort = "xhigh"

[projects."/root/code/ais"]
trust_level = "trusted"
"#,
        )
        .unwrap();

        let profile = CodexProfile {
            auth: serde_json::json!({ "OPENAI_API_KEY": "abc" }),
            auth_config: Some(AuthConfig {
                model_provider: Some("api111".to_string()),
                preferred_auth_method: Some("apikey".to_string()),
                model_providers: BTreeMap::from([(
                    "api111".to_string(),
                    ProviderConfig {
                        name: "api111".to_string(),
                        base_url: "https://api.example.com".to_string(),
                        wire_api: "responses".to_string(),
                        supports_websockets: None,
                        requires_openai_auth: None,
                    },
                )]),
                features: FeatureConfig::default(),
            }),
        };
        apply_profile(&profile, &auth_path, &config_path).unwrap();

        let auth: JsonValue =
            serde_json::from_str(&fs::read_to_string(auth_path).unwrap()).unwrap();
        assert_eq!(auth["OPENAI_API_KEY"], "abc");

        let config = fs::read_to_string(config_path).unwrap();
        assert!(config.contains(r#"model = "gpt-5.5""#));
        assert!(config.contains(r#"model_provider = "api111""#));
        assert!(config.contains(r#"preferred_auth_method = "apikey""#));
        assert!(config.contains(r#"[model_providers.api111]"#));
        assert!(config.contains(r#"base_url = "https://api.example.com""#));
        assert!(config.contains(r#"[projects."/root/code/ais"]"#));
    }

    #[test]
    fn capture_auth_config_reads_provider_tables() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("config.toml");
        fs::write(
            &config_path,
            r#"model = "gpt-5.5"
model_provider = "api111"
preferred_auth_method = "apikey"

[model_providers.api111]
name = "api111"
base_url = "https://api.example.com"
wire_api = "responses"
"#,
        )
        .unwrap();

        let config = capture_auth_config(&config_path).unwrap().unwrap();
        assert_eq!(config.model_provider.as_deref(), Some("api111"));
        assert_eq!(config.preferred_auth_method.as_deref(), Some("apikey"));
        assert_eq!(
            config.model_providers["api111"],
            ProviderConfig {
                name: "api111".to_string(),
                base_url: "https://api.example.com".to_string(),
                wire_api: "responses".to_string(),
                supports_websockets: None,
                requires_openai_auth: None
            }
        );
        assert!(config.features.is_empty());
    }

    #[test]
    fn create_provider_profile_derives_name_from_base_url() {
        let (name, profile) =
            create_provider_profile(None, "https://api.example.test/v1", "xxkey", false).unwrap();

        assert_eq!(name, "example");
        assert_eq!(
            profile.auth,
            serde_json::json!({ "OPENAI_API_KEY": "xxkey" })
        );

        let auth_config = profile.auth_config.unwrap();
        assert_eq!(auth_config.model_provider.as_deref(), Some("example"));
        assert_eq!(auth_config.preferred_auth_method.as_deref(), Some("apikey"));
        assert_eq!(
            auth_config.model_providers["example"],
            ProviderConfig {
                name: "example".to_string(),
                base_url: "https://api.example.test/v1".to_string(),
                wire_api: "responses".to_string(),
                supports_websockets: None,
                requires_openai_auth: Some(true)
            }
        );
        assert!(auth_config.features.is_empty());
    }

    #[test]
    fn create_provider_profile_uses_explicit_name() {
        let (name, profile) = create_provider_profile(
            Some("Example API"),
            "https://api.example.com/v1",
            "xxkey",
            false,
        )
        .unwrap();

        assert_eq!(name, "example-api");
        assert_eq!(
            profile.auth_config.unwrap().model_providers["example-api"].base_url,
            "https://api.example.com/v1"
        );
    }

    #[test]
    fn capture_auth_config_reads_websocket_fields() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("config.toml");
        fs::write(
            &config_path,
            r#"model = "gpt-5.5"
model_provider = "OpenAI"
preferred_auth_method = "apikey"

[model_providers.OpenAI]
name = "OpenAI"
base_url = "https://api.example.com"
wire_api = "responses"
supports_websockets = true
requires_openai_auth = true

[features]
responses_websockets_v2 = true
goals = true
"#,
        )
        .unwrap();

        let config = capture_auth_config(&config_path).unwrap().unwrap();
        assert_eq!(config.model_provider.as_deref(), Some("OpenAI"));
        assert_eq!(
            config.model_providers["OpenAI"],
            ProviderConfig {
                name: "OpenAI".to_string(),
                base_url: "https://api.example.com".to_string(),
                wire_api: "responses".to_string(),
                supports_websockets: Some(true),
                requires_openai_auth: Some(true),
            }
        );
        assert_eq!(config.features.responses_websockets_v2, Some(true));
    }

    #[test]
    fn switch_to_websocket_profile_writes_websocket_config() {
        let dir = tempdir().unwrap();
        let auth_path = dir.path().join("auth.json");
        let config_path = dir.path().join("config.toml");
        fs::write(
            &config_path,
            r#"model = "gpt-5.5"

[features]
goals = true
"#,
        )
        .unwrap();

        let (_, profile) =
            create_provider_profile(Some("OpenAI"), "https://api.example.com", "xxkey", true)
                .unwrap();
        apply_profile(&profile, &auth_path, &config_path).unwrap();

        let config = fs::read_to_string(config_path).unwrap();
        assert!(config.contains(r#"supports_websockets = true"#));
        assert!(config.contains(r#"requires_openai_auth = true"#));
        assert!(config.contains(r#"responses_websockets_v2 = true"#));
        assert!(config.contains(r#"goals = true"#));
    }

    #[test]
    fn switch_to_non_websocket_profile_removes_websocket_feature_only() {
        let dir = tempdir().unwrap();
        let auth_path = dir.path().join("auth.json");
        let config_path = dir.path().join("config.toml");
        fs::write(
            &config_path,
            r#"model = "gpt-5.5"
model_provider = "OpenAI"
preferred_auth_method = "apikey"

[model_providers.OpenAI]
name = "OpenAI"
base_url = "https://api.example.com"
wire_api = "responses"
supports_websockets = true
requires_openai_auth = true

[features]
responses_websockets_v2 = true
goals = true
"#,
        )
        .unwrap();

        let profile = CodexProfile {
            auth: serde_json::json!({ "OPENAI_API_KEY": "xxkey" }),
            auth_config: Some(AuthConfig {
                model_provider: Some("OpenAI".to_string()),
                preferred_auth_method: Some("apikey".to_string()),
                model_providers: BTreeMap::from([(
                    "OpenAI".to_string(),
                    ProviderConfig {
                        name: "OpenAI".to_string(),
                        base_url: "https://api.example.com".to_string(),
                        wire_api: "responses".to_string(),
                        supports_websockets: None,
                        requires_openai_auth: Some(true),
                    },
                )]),
                features: FeatureConfig::default(),
            }),
        };
        apply_profile(&profile, &auth_path, &config_path).unwrap();

        let config = fs::read_to_string(config_path).unwrap();
        assert!(!config.contains(r#"supports_websockets = true"#));
        assert!(!config.contains(r#"responses_websockets_v2 = true"#));
        assert!(config.contains(r#"requires_openai_auth = true"#));
        assert!(config.contains(r#"goals = true"#));
    }
}
