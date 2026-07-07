use super::{
    delete_profile,
    env::{capture_profile_from, render_env_exports},
    profile::{create_profile, ModelOptions},
};
use crate::{
    cli::Cli,
    store::{load_store, save_store, Store},
};
use clap::Parser;
use std::collections::BTreeMap;
use tempfile::tempdir;

#[test]
fn create_profile_derives_name_from_base_url() {
    let (name, profile) = create_profile(
        None,
        "https://api.example.test/v1",
        "example-token",
        ModelOptions::default(),
    )
    .unwrap();

    assert_eq!(name, "example");
    assert_eq!(profile.base_url(), "https://api.example.test/v1");
    assert_eq!(profile.auth_token(), "example-token");
    assert!(profile.disable_nonessential_traffic());
    assert!(!profile.attribution_header());
}

#[test]
fn create_profile_uses_explicit_name() {
    let (name, profile) = create_profile(
        Some("Example API"),
        "https://api.example.com/v1",
        "example-token",
        ModelOptions::default(),
    )
    .unwrap();

    assert_eq!(name, "example-api");
    assert_eq!(profile.base_url(), "https://api.example.com/v1");
}

#[test]
fn store_round_trip_preserves_claude_profiles() {
    let dir = tempdir().unwrap();
    let store_path = dir.path().join("store.json");
    let mut store = Store::default();
    let (_, profile) = create_profile(
        Some("example"),
        "https://api.example.com/v1",
        "example-token",
        ModelOptions::default(),
    )
    .unwrap();
    store.claude.insert("example".to_string(), profile);

    save_store(&store_path, &store).unwrap();

    let store = load_store(&store_path).unwrap();
    let profile = store.claude.get("example").unwrap();
    assert_eq!(profile.base_url(), "https://api.example.com/v1");
}

#[test]
fn legacy_store_without_claude_profiles_still_loads() {
    let dir = tempdir().unwrap();
    let store_path = dir.path().join("store.json");
    std::fs::write(&store_path, r#"{"version":1,"codex":{}}"#).unwrap();

    let store = load_store(&store_path).unwrap();
    assert!(store.claude.is_empty());
}

#[test]
fn delete_profile_removes_saved_profile() {
    let dir = tempdir().unwrap();
    let store_path = dir.path().join("store.json");
    let (_, first) = create_profile(
        Some("first"),
        "https://api.example.com/v1",
        "first-token",
        ModelOptions::default(),
    )
    .unwrap();
    let (_, second) = create_profile(
        Some("second"),
        "https://api.example.test/v1",
        "second-token",
        ModelOptions::default(),
    )
    .unwrap();
    let store = Store {
        version: 1,
        claude: BTreeMap::from([("first".to_string(), first), ("second".to_string(), second)]),
        codex: BTreeMap::new(),
    };
    save_store(&store_path, &store).unwrap();

    delete_profile(&store_path, "first").unwrap();

    let store = load_store(&store_path).unwrap();
    assert!(!store.claude.contains_key("first"));
    assert!(store.claude.contains_key("second"));
}

#[test]
fn delete_profile_errors_for_missing_profile() {
    let dir = tempdir().unwrap();
    let store_path = dir.path().join("store.json");
    save_store(&store_path, &Store::default()).unwrap();

    let error = delete_profile(&store_path, "missing").unwrap_err();
    assert_eq!(
        error.to_string(),
        "claude profile 'missing' not found; run `ais claude list` to see saved profiles"
    );
}

#[test]
fn delete_command_requires_profile_name() {
    let error = match Cli::try_parse_from(["ais", "claude", "delete"]) {
        Ok(_) => panic!("delete without a profile name should fail"),
        Err(error) => error,
    };
    assert_eq!(
        error.kind(),
        clap::error::ErrorKind::MissingRequiredArgument
    );
}

#[test]
fn env_command_requires_profile_name() {
    let error = match Cli::try_parse_from(["ais", "claude", "env"]) {
        Ok(_) => panic!("env without a profile name should fail"),
        Err(error) => error,
    };
    assert_eq!(
        error.kind(),
        clap::error::ErrorKind::MissingRequiredArgument
    );
}

#[test]
fn create_profile_applies_default_model_to_primary_opus_and_sonnet() {
    let (_, profile) = create_profile(
        Some("example"),
        "https://api.example.com/anthropic",
        "example-token",
        ModelOptions {
            default_model: Some("example-pro".to_string()),
            default_haiku_model: Some("example-flash".to_string()),
            subagent_model: Some("example-flash".to_string()),
            effort_level: Some("max".to_string()),
            ..ModelOptions::default()
        },
    )
    .unwrap();

    assert_eq!(profile.model(), Some("example-pro"));
    assert_eq!(profile.default_opus_model(), Some("example-pro"));
    assert_eq!(profile.default_sonnet_model(), Some("example-pro"));
    assert_eq!(profile.default_haiku_model(), Some("example-flash"));
    assert_eq!(profile.subagent_model(), Some("example-flash"));
    assert_eq!(profile.effort_level(), Some("max"));
}

#[test]
fn create_profile_specific_models_override_default_model() {
    let (_, profile) = create_profile(
        Some("example"),
        "https://api.example.com/anthropic",
        "example-token",
        ModelOptions {
            default_model: Some("example-pro".to_string()),
            model: Some("example-primary".to_string()),
            default_opus_model: Some("example-opus".to_string()),
            default_sonnet_model: Some("example-sonnet".to_string()),
            ..ModelOptions::default()
        },
    )
    .unwrap();

    assert_eq!(profile.model(), Some("example-primary"));
    assert_eq!(profile.default_opus_model(), Some("example-opus"));
    assert_eq!(profile.default_sonnet_model(), Some("example-sonnet"));
}

#[test]
fn render_env_exports_includes_model_fields_when_present() {
    let (_, profile) = create_profile(
        Some("example"),
        "https://api.example.com/anthropic",
        "example-token",
        ModelOptions {
            default_model: Some("example-pro".to_string()),
            default_haiku_model: Some("example-flash".to_string()),
            subagent_model: Some("example-flash".to_string()),
            effort_level: Some("max".to_string()),
            ..ModelOptions::default()
        },
    )
    .unwrap();

    let exports = render_env_exports(&profile);
    assert!(exports.contains("export ANTHROPIC_MODEL='example-pro'"));
    assert!(exports.contains("export ANTHROPIC_DEFAULT_OPUS_MODEL='example-pro'"));
    assert!(exports.contains("export ANTHROPIC_DEFAULT_SONNET_MODEL='example-pro'"));
    assert!(exports.contains("export ANTHROPIC_DEFAULT_HAIKU_MODEL='example-flash'"));
    assert!(exports.contains("export CLAUDE_CODE_SUBAGENT_MODEL='example-flash'"));
    assert!(exports.contains("export CLAUDE_CODE_EFFORT_LEVEL='max'"));
    assert!(exports.contains("export CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC=1"));
    assert!(exports.contains("export CLAUDE_CODE_ATTRIBUTION_HEADER=0"));
}

#[test]
fn capture_profile_from_env_reads_minimal_required_values() {
    let profile = capture_profile_from(|name| match name {
        "ANTHROPIC_BASE_URL" => Some("https://api.example.com/anthropic".to_string()),
        "ANTHROPIC_AUTH_TOKEN" => Some("example-token".to_string()),
        _ => None,
    })
    .unwrap();

    assert_eq!(profile.base_url(), "https://api.example.com/anthropic");
    assert_eq!(profile.auth_token(), "example-token");
    assert!(profile.disable_nonessential_traffic());
    assert!(!profile.attribution_header());
}

#[test]
fn capture_profile_from_env_reads_full_model_config() {
    let profile = capture_profile_from(|name| match name {
        "ANTHROPIC_BASE_URL" => Some("https://api.example.com/anthropic".to_string()),
        "ANTHROPIC_AUTH_TOKEN" => Some("example-token".to_string()),
        "ANTHROPIC_MODEL" => Some("example-pro".to_string()),
        "ANTHROPIC_DEFAULT_OPUS_MODEL" => Some("example-opus".to_string()),
        "ANTHROPIC_DEFAULT_SONNET_MODEL" => Some("example-sonnet".to_string()),
        "ANTHROPIC_DEFAULT_HAIKU_MODEL" => Some("example-haiku".to_string()),
        "CLAUDE_CODE_SUBAGENT_MODEL" => Some("example-subagent".to_string()),
        "CLAUDE_CODE_EFFORT_LEVEL" => Some("max".to_string()),
        "CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC" => Some("false".to_string()),
        "CLAUDE_CODE_ATTRIBUTION_HEADER" => Some("yes".to_string()),
        _ => None,
    })
    .unwrap();

    assert_eq!(profile.model(), Some("example-pro"));
    assert_eq!(profile.default_opus_model(), Some("example-opus"));
    assert_eq!(profile.default_sonnet_model(), Some("example-sonnet"));
    assert_eq!(profile.default_haiku_model(), Some("example-haiku"));
    assert_eq!(profile.subagent_model(), Some("example-subagent"));
    assert_eq!(profile.effort_level(), Some("max"));
    assert!(!profile.disable_nonessential_traffic());
    assert!(profile.attribution_header());
}

#[test]
fn capture_profile_from_env_errors_without_base_url() {
    let error = capture_profile_from(|name| match name {
        "ANTHROPIC_AUTH_TOKEN" => Some("example-token".to_string()),
        _ => None,
    })
    .unwrap_err();

    assert_eq!(error.to_string(), "ANTHROPIC_BASE_URL is not set");
}

#[test]
fn capture_profile_from_env_errors_without_auth_token() {
    let error = capture_profile_from(|name| match name {
        "ANTHROPIC_BASE_URL" => Some("https://api.example.com/anthropic".to_string()),
        _ => None,
    })
    .unwrap_err();

    assert_eq!(error.to_string(), "ANTHROPIC_AUTH_TOKEN is not set");
}

#[test]
fn capture_profile_from_env_errors_for_invalid_bool() {
    let error = capture_profile_from(|name| match name {
        "ANTHROPIC_BASE_URL" => Some("https://api.example.com/anthropic".to_string()),
        "ANTHROPIC_AUTH_TOKEN" => Some("example-token".to_string()),
        "CLAUDE_CODE_ATTRIBUTION_HEADER" => Some("maybe".to_string()),
        _ => None,
    })
    .unwrap_err();

    assert_eq!(
        error.to_string(),
        "CLAUDE_CODE_ATTRIBUTION_HEADER must be one of true, false, 1, 0, yes, no, on, off"
    );
}

#[test]
fn save_command_requires_profile_name() {
    let error = match Cli::try_parse_from(["ais", "claude", "save"]) {
        Ok(_) => panic!("save without a profile name should fail"),
        Err(error) => error,
    };
    assert_eq!(
        error.kind(),
        clap::error::ErrorKind::MissingRequiredArgument
    );
}

#[test]
fn create_command_accepts_model_options() {
    let parsed = Cli::try_parse_from([
        "ais",
        "claude",
        "create",
        "--name",
        "example",
        "--default-model",
        "example-pro",
        "--haiku-model",
        "example-flash",
        "--subagent-model",
        "example-flash",
        "--effort-level",
        "max",
        "https://api.example.com/anthropic",
        "example-token",
    ]);

    assert!(parsed.is_ok());
}
