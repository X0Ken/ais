use anyhow::{bail, Context, Result};

pub(crate) fn provider_name_from_base_url(base_url: &str) -> Result<String> {
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

pub(crate) fn normalize_provider_name(name: &str) -> Result<String> {
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
