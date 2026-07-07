use anyhow::{bail, Result};

use super::profile::{build_profile, ClaudeProfile, ModelOptions, TrafficOptions};

const ANTHROPIC_BASE_URL_ENV: &str = "ANTHROPIC_BASE_URL";
const ANTHROPIC_AUTH_TOKEN_ENV: &str = "ANTHROPIC_AUTH_TOKEN";
const ANTHROPIC_MODEL_ENV: &str = "ANTHROPIC_MODEL";
const ANTHROPIC_DEFAULT_OPUS_MODEL_ENV: &str = "ANTHROPIC_DEFAULT_OPUS_MODEL";
const ANTHROPIC_DEFAULT_SONNET_MODEL_ENV: &str = "ANTHROPIC_DEFAULT_SONNET_MODEL";
const ANTHROPIC_DEFAULT_HAIKU_MODEL_ENV: &str = "ANTHROPIC_DEFAULT_HAIKU_MODEL";
const CLAUDE_CODE_SUBAGENT_MODEL_ENV: &str = "CLAUDE_CODE_SUBAGENT_MODEL";
const CLAUDE_CODE_EFFORT_LEVEL_ENV: &str = "CLAUDE_CODE_EFFORT_LEVEL";
const CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC_ENV: &str =
    "CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC";
const CLAUDE_CODE_ATTRIBUTION_HEADER_ENV: &str = "CLAUDE_CODE_ATTRIBUTION_HEADER";

pub(super) fn print_env_exports(profile: &ClaudeProfile) {
    print!("{}", render_env_exports(profile));
}

pub(super) fn capture_current_profile() -> Result<ClaudeProfile> {
    capture_profile_from(|name| std::env::var(name).ok())
}

pub(super) fn capture_profile_from<F>(mut get_env: F) -> Result<ClaudeProfile>
where
    F: FnMut(&str) -> Option<String>,
{
    let base_url = required_env(&mut get_env, ANTHROPIC_BASE_URL_ENV)?;
    let auth_token = required_env(&mut get_env, ANTHROPIC_AUTH_TOKEN_ENV)?;
    let traffic = TrafficOptions {
        disable_nonessential_traffic: bool_env(
            &mut get_env,
            CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC_ENV,
            true,
        )?,
        attribution_header: bool_env(&mut get_env, CLAUDE_CODE_ATTRIBUTION_HEADER_ENV, false)?,
    };
    let models = ModelOptions {
        model: optional_env(&mut get_env, ANTHROPIC_MODEL_ENV),
        default_model: None,
        default_opus_model: optional_env(&mut get_env, ANTHROPIC_DEFAULT_OPUS_MODEL_ENV),
        default_sonnet_model: optional_env(&mut get_env, ANTHROPIC_DEFAULT_SONNET_MODEL_ENV),
        default_haiku_model: optional_env(&mut get_env, ANTHROPIC_DEFAULT_HAIKU_MODEL_ENV),
        subagent_model: optional_env(&mut get_env, CLAUDE_CODE_SUBAGENT_MODEL_ENV),
        effort_level: optional_env(&mut get_env, CLAUDE_CODE_EFFORT_LEVEL_ENV),
    };

    build_profile(&base_url, &auth_token, traffic, models)
}

pub(super) fn render_env_exports(profile: &ClaudeProfile) -> String {
    let mut lines = Vec::new();
    push_export(
        &mut lines,
        ANTHROPIC_BASE_URL_ENV,
        shell_quote(profile.base_url()),
    );
    push_export(
        &mut lines,
        ANTHROPIC_AUTH_TOKEN_ENV,
        shell_quote(profile.auth_token()),
    );
    push_optional_export(&mut lines, ANTHROPIC_MODEL_ENV, profile.model());
    push_optional_export(
        &mut lines,
        ANTHROPIC_DEFAULT_OPUS_MODEL_ENV,
        profile.default_opus_model(),
    );
    push_optional_export(
        &mut lines,
        ANTHROPIC_DEFAULT_SONNET_MODEL_ENV,
        profile.default_sonnet_model(),
    );
    push_optional_export(
        &mut lines,
        ANTHROPIC_DEFAULT_HAIKU_MODEL_ENV,
        profile.default_haiku_model(),
    );
    push_optional_export(
        &mut lines,
        CLAUDE_CODE_SUBAGENT_MODEL_ENV,
        profile.subagent_model(),
    );
    push_optional_export(
        &mut lines,
        CLAUDE_CODE_EFFORT_LEVEL_ENV,
        profile.effort_level(),
    );
    push_export(
        &mut lines,
        CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC_ENV,
        bool_as_env(profile.disable_nonessential_traffic()).to_string(),
    );
    push_export(
        &mut lines,
        CLAUDE_CODE_ATTRIBUTION_HEADER_ENV,
        bool_as_env(profile.attribution_header()).to_string(),
    );

    format!("{}\n", lines.join("\n"))
}

fn push_optional_export(lines: &mut Vec<String>, name: &'static str, value: Option<&str>) {
    if let Some(value) = value {
        push_export(lines, name, shell_quote(value));
    }
}

fn push_export(lines: &mut Vec<String>, name: &'static str, value: String) {
    lines.push(format!("export {name}={value}"));
}

fn required_env<F>(get_env: &mut F, name: &'static str) -> Result<String>
where
    F: FnMut(&str) -> Option<String>,
{
    let Some(value) = get_env(name) else {
        bail!("{name} is not set");
    };
    let value = value.trim();
    if value.is_empty() {
        bail!("{name} is not set");
    }
    Ok(value.to_string())
}

fn optional_env<F>(get_env: &mut F, name: &'static str) -> Option<String>
where
    F: FnMut(&str) -> Option<String>,
{
    get_env(name)
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn bool_env<F>(get_env: &mut F, name: &'static str, default: bool) -> Result<bool>
where
    F: FnMut(&str) -> Option<String>,
{
    let Some(value) = get_env(name) else {
        return Ok(default);
    };
    parse_bool(name, value.trim())
}

fn parse_bool(name: &'static str, value: &str) -> Result<bool> {
    match value.to_ascii_lowercase().as_str() {
        "true" | "1" | "yes" | "on" => Ok(true),
        "false" | "0" | "no" | "off" => Ok(false),
        _ => bail!("{name} must be one of true, false, 1, 0, yes, no, on, off"),
    }
}

fn bool_as_env(value: bool) -> &'static str {
    if value {
        "1"
    } else {
        "0"
    }
}

fn shell_quote(value: &str) -> String {
    if value.is_empty() {
        return "''".to_string();
    }

    let mut quoted = String::from("'");
    for ch in value.chars() {
        if ch == '\'' {
            quoted.push_str("'\\''");
        } else {
            quoted.push(ch);
        }
    }
    quoted.push('\'');
    quoted
}

#[cfg(test)]
mod unit_tests {
    use super::shell_quote;

    #[test]
    fn shell_quote_wraps_plain_values() {
        assert_eq!(shell_quote("abc"), "'abc'");
    }

    #[test]
    fn shell_quote_escapes_single_quotes() {
        assert_eq!(shell_quote("a'b"), "'a'\\''b'");
    }
}
