# ais

`ais` is a small command-line tool for switching AI agent authentication
profiles. It currently supports Codex authentication profiles.

The tool stores saved authentication profiles in one JSON file instead of using
multiple `/root/.codex.*` directories.

## Install

From this repository:

```bash
cargo install --path .
```

Make sure Cargo's bin directory is in your `PATH`:

```bash
export PATH="$HOME/.cargo/bin:$PATH"
```

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

Switch Codex to a saved profile:

```bash
ais codex switch <name>
```

Examples:

```bash
ais codex switch openai
ais codex switch wan
```

`switch` works the same way for OpenAI login profiles and API key profiles.

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

Other Codex configuration is preserved.
