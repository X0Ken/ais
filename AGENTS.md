# AGENTS.md

## Project Rules

- Use generic example placeholders in committed code, docs, tests, and snippets. Do not include real provider names, gateway names, customer/vendor names, live service URLs, API keys, tokens, or other secrets. Prefer names such as `example` or `example-api`, URLs such as `https://api.example.com/v1`, and values such as `<api-key>`.
- Keep Rust code split by responsibility. `src/main.rs` should stay as the small CLI entry point; move command implementations, storage, path handling, naming helpers, config editing, and tests into focused modules. When a file grows large or takes on a second responsibility, split it before adding more behavior.
- Keep the crate version in sync with the release tag. The `ais --version` output comes from `Cargo.toml`'s `version` (read by clap via `CARGO_PKG_VERSION`), and the `release.yml` workflow builds from the pushed `v*` tag. Before creating a new tag `vX.Y.Z`, bump `version` in `Cargo.toml` (and the `ais` entry in `Cargo.lock`) to the matching `X.Y.Z` and commit it, so the released binary reports the same version as its tag.
