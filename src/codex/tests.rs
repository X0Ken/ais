use super::{
    config::capture_auth_config,
    delete_codex_profile,
    profile::{
        apply_profile, create_provider_profile, AuthConfig, CodexProfile, FeatureConfig,
        ProviderConfig,
    },
};
use crate::{
    cli::Cli,
    store::{load_store, save_store, Store},
};
use clap::Parser;
use serde_json::Value as JsonValue;
use std::{collections::BTreeMap, fs};
use tempfile::tempdir;
use toml_edit::DocumentMut;

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

    let auth: JsonValue = serde_json::from_str(&fs::read_to_string(auth_path).unwrap()).unwrap();
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
fn delete_codex_profile_removes_saved_profile() {
    let dir = tempdir().unwrap();
    let store_path = dir.path().join("store.json");
    let store = Store {
        version: 1,
        claude: BTreeMap::new(),
        codex: BTreeMap::from([
            (
                "first".to_string(),
                CodexProfile {
                    auth: serde_json::json!({ "OPENAI_API_KEY": "first-key" }),
                    auth_config: None,
                },
            ),
            (
                "second".to_string(),
                CodexProfile {
                    auth: serde_json::json!({ "OPENAI_API_KEY": "second-key" }),
                    auth_config: None,
                },
            ),
        ]),
    };
    save_store(&store_path, &store).unwrap();

    delete_codex_profile(&store_path, "first").unwrap();

    let store = load_store(&store_path).unwrap();
    assert!(!store.codex.contains_key("first"));
    assert!(store.codex.contains_key("second"));
}

#[test]
fn delete_codex_profile_errors_for_missing_profile() {
    let dir = tempdir().unwrap();
    let store_path = dir.path().join("store.json");
    save_store(&store_path, &Store::default()).unwrap();

    let error = delete_codex_profile(&store_path, "missing").unwrap_err();
    assert_eq!(
        error.to_string(),
        "codex profile 'missing' not found; run `ais codex list` to see saved profiles"
    );
}

#[test]
fn delete_command_requires_profile_name() {
    let error = match Cli::try_parse_from(["ais", "codex", "delete"]) {
        Ok(_) => panic!("delete without a profile name should fail"),
        Err(error) => error,
    };
    assert_eq!(
        error.kind(),
        clap::error::ErrorKind::MissingRequiredArgument
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
        create_provider_profile(Some("OpenAI"), "https://api.example.com", "xxkey", true).unwrap();
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
