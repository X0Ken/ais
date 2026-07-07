# ais

`ais` is a small command-line tool for switching AI agent authentication
profiles. It supports Codex authentication profiles and Claude Code
environment profiles.

## Install

From this repository:

```bash
cargo install --path .
```

Make sure Cargo's bin directory is in your `PATH`:

```bash
export PATH="$HOME/.cargo/bin:$PATH"
```

Linux release binaries are built with the `x86_64-unknown-linux-musl` target.

## Usage

List saved Codex profiles:

```bash
ais codex list
```

Save the current `~/.codex` authentication as a named profile:

```bash
ais codex save <name>
```

Create and switch to a Codex API key provider profile:

```bash
ais codex create <base-url> <api-key>
```

You can choose the saved profile/provider name explicitly:

```bash
ais codex create --name <name> <base-url> <api-key>
```

Enable Codex responses websocket support for a provider profile:

```bash
ais codex create --websocket --name <name> <base-url> <api-key>
ais codex create --ws --name <name> <base-url> <api-key>
```

Switch Codex to a saved profile:

```bash
ais codex switch <name>
```

Delete a saved Codex profile:

```bash
ais codex delete <name>
```

Examples:

```bash
ais codex switch example-login
ais codex switch example-api
```

`switch` works the same way for OpenAI login profiles and API key profiles.

## Claude Code

Create a Claude Code environment profile:

```bash
ais claude create https://api.example.com/v1 <auth-token>
```

You can choose the saved profile name explicitly:

```bash
ais claude create --name example https://api.example.com/v1 <auth-token>
```

For custom model providers, include model environment settings in the saved
profile:

```bash
ais claude create \
  --name example \
  --default-model example-pro \
  --haiku-model example-flash \
  --subagent-model example-flash \
  --effort-level max \
  https://api.example.com/anthropic \
  <auth-token>
```

`--default-model` sets `ANTHROPIC_MODEL`,
`ANTHROPIC_DEFAULT_OPUS_MODEL`, and `ANTHROPIC_DEFAULT_SONNET_MODEL`
together. More specific flags such as `--model`, `--opus-model`, and
`--sonnet-model` override it.

Save the current Claude Code environment variables as a named profile:

```bash
ais claude save example
```

Apply a saved Claude Code environment profile to the current shell:

```bash
eval "$(ais claude env example)"
```

`env` prints shell exports for:

```text
ANTHROPIC_BASE_URL
ANTHROPIC_AUTH_TOKEN
ANTHROPIC_MODEL
ANTHROPIC_DEFAULT_OPUS_MODEL
ANTHROPIC_DEFAULT_SONNET_MODEL
ANTHROPIC_DEFAULT_HAIKU_MODEL
CLAUDE_CODE_SUBAGENT_MODEL
CLAUDE_CODE_EFFORT_LEVEL
CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC
CLAUDE_CODE_ATTRIBUTION_HEADER
```

List saved Claude Code profiles:

```bash
ais claude list
```

Delete a saved Claude Code profile:

```bash
ais claude delete <name>
```

## Storage

By default, profiles are stored at:

```text
~/.config/ais/codex-auth.json
```

You can override the store path with:

```bash
AIS_STORE=/path/to/codex-auth.json ais codex list
```

For testing or migration, you can point the Codex home directory at another
location:

```bash
AIS_CODEX_HOME=/path/to/.codex ais codex save <name>
```

During switching, `ais` only updates Codex authentication-related files and
settings:

- `auth.json`
- `model_provider`
- `preferred_auth_method`
- the selected provider entry under `model_providers`
- `responses_websockets_v2` under `features`

Other Codex configuration is preserved.
