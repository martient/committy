# Committy agent guide

Committy is a Rust CLI for conventional commits, branch policy, grouped commits, releases, and multi-package version updates. The checkout is CLI-first; do not assume a TUI exists.

## Work safely

- Discover active policy with `cargo run -- schema --output json`.
- Prefer `--non-interactive`, `--repo-path`, `--dry-run`, and `--output json`.
- Inspect `api_version`, `ok`, `errors`, and the command-specific plan before applying it.
- Use the `committy-branch`, `committy-commit`, `committy-release`, and `committy-enforce` plugin skills when available.
- Never infer permission for a push, publish, destructive Git action, or unrelated staging.

## Build and verify

```bash
cargo check
cargo test
cargo fmt --all -- --check
cargo clippy --all-targets --all-features
```

Tests are in `tests/`; CLI integration tests use `assert_cmd` and temporary repositories. Add public-seam tests for machine output, exit codes, Git behavior, hook protocol, and plugin manifests.

## Architecture

- `src/cli/commands/`: command implementations and JSON payloads
- `src/config/`, `src/convention/`: hierarchical typed policy
- `src/git/`: repository and Git operations
- `src/linter/`: message/history validation; exit code 3 means lint findings
- `src/workflow/`, `src/versioning/`, `src/packages/`: release side effects
- `plugins/committy/`: shared Codex and Claude skills

PRs target `develop`. Commits follow Conventional Commits. The canonical agent reference is `docs/src/content/docs/reference/agent-workflows.mdx`.
