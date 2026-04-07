# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Overview

Committy is a Rust CLI for composing conventional commits, linting history quality, planning grouped commits, generating tags, and orchestrating multi-package version updates.

The current repository state is **CLI-first and JSON-friendly**. This checkout does **not** contain a checked-in `src/tui/` module or a `tui` subcommand, so Claude workflows should use the machine-readable CLI commands instead of assuming a terminal UI exists.

## Essential Commands

### Build and Test
```bash
# Build the project
cargo build

# Run all tests
cargo test

# Run a specific test
cargo test test_name

# Run tests with verbose output
cargo test -- --nocapture
```

### Running the CLI
```bash
# Run from source
cargo run

# Run with arguments
cargo run -- <subcommand> [options]

# Agent-safe previews
cargo run -- branch --type feat --ticket AI42 --subject "agent flow" --dry-run --output json
cargo run -- commit --type feat --scope core --message "add agent flow" --dry-run --output json
cargo run -- group-commit --mode plan --output json
cargo run -- lint --output json
cargo run -- tag --dry-run --output json
```

### Development
```bash
# Check code without building
cargo check

# Format code
cargo fmt

# Run linter
cargo clippy
```

## Current CLI Surface

The current checked-in command set is defined in `src/cli/mod.rs`:
- `commit`
- `amend`
- `tag`
- `lint`
- `lint-message`
- `branch`
- `group-commit`
- `init`
- `config`
- `packages`

## Architecture

### Module Structure

**CLI Layer** (`src/cli/`)
- `commands/`: concrete command implementations for `amend`, `branch`, `commit`, `config`, `group_commit`, `init`, `lint`, `lint_message`, `packages`, and `tag`
- `src/main.rs`: top-level flag parsing, non-interactive mode detection, update checks, and command dispatch

**Workflow + Repository Config**
- `.committy/config.toml`: repository-level multi-package config for Committy itself
- `src/workflow/orchestrator.rs`: coordinates scope detection, version bumps, and dependency updates
- `src/config/hierarchy.rs` and `src/config/repository.rs`: merged config loading and repository config support
- Current scope mapping in this repo:
  - `src/**` → `core`
  - `docs/**` → `docs`

**Git Operations** (`src/git/`)
- `repository.rs`: repository discovery, staged-file queries, and git config validation
- `commit.rs`: commit creation and message formatting
- `tag.rs`: tag generation helpers
- `branch.rs`: branch operations

**Linting** (`src/linter/`)
- Validates conventional commit messages
- Used by `commit`, `amend`, `group-commit`, `lint`, and `lint-message`
- Exit code `3` is reserved for lint issues

**Packages + Versioning** (`src/packages/`, `src/versioning/`, `src/version/`)
- Multi-package detection and sync helpers
- Version bump calculation for independent, unified, and hybrid strategies
- Version file updates across supported package managers

**AI Integration** (`src/ai/`)
- Supports OpenRouter and Ollama providers
- Currently used by `group-commit --ai`

**Input/Prompts** (`src/input/`)
- Interactive prompt handling via `inquire`
- Validation and auto-correction helpers for commit and branch inputs

**Error Handling** (`src/error.rs`)
- `CliError` is the central error type across commands

### Key Flow: Commit Command

1. `src/cli/commands/commit.rs` discovers the repo and checks staged changes.
2. It loads allowed commit types and, when repo config exists, auto-detects scopes.
3. It formats and lints the full commit message.
4. When `.committy/config.toml` is available, it runs `WorkflowOrchestrator` to preview or apply version/dependency side effects.
5. `--dry-run --output json` returns the resolved message plus an optional `workflow` preview without mutating git.
6. A real run validates git config, stages workflow-modified files if needed, and then creates or amends the commit.

### Key Flow: Group Commit

1. `src/cli/commands/group_commit.rs` classifies changed files into `docs`, `tests`, `ci`, `deps`, `build`, `chore`, and `code` groups.
2. `--mode plan` returns grouped JSON output without mutations.
3. `--mode apply` creates one validated commit per group.
4. `--push` is only valid in apply mode and must be paired with `--confirm-push`.

## Recommended Agent Workflow

Use the current CLI states as a **preview → inspect → apply** workflow:

1. Preview first with `--dry-run --output json` when the command supports it.
2. Inspect `ok`, `dry_run`, `errors`, and command-specific fields such as:
   - `branch_name` for `branch`
   - `message`, `commit_type`, `scope`, and optional `workflow` for `commit`
   - `mode`, `groups`, and `commits` for `group-commit`
3. Rerun without `--dry-run` only when the preview matches the intended change.
4. Use `--repo-path` when operating on a checkout other than the current shell directory.
5. Before any push or release, run `committy --non-interactive lint --repo-path . --output json`.
6. Require explicit confirmation for remote mutations:
   - `group-commit --mode apply --push --confirm-push`
   - `tag --publish --confirm-publish`

The canonical reference for this flow is `docs/src/content/docs/reference/agent-workflows.mdx`.

## Non-Interactive Mode

- Enabled via `--non-interactive`, `COMMITTY_NONINTERACTIVE=1`, or `CI=1`
- In non-interactive mode, required flags must be provided and prompts are skipped
- This is the preferred mode for agentic or CI-driven usage

## Build System

- `build.rs` injects `SENTRY_DSN` and `POSTHOG_API_KEY` at compile time from environment variables
- Defaults to `"undefined"` if those environment variables are not set

## Testing

- Tests live in `tests/`
- Integration-style CLI coverage uses `assert_cmd`
- Temporary git repositories are created with `tempfile`
- Good entry points for agent workflow behavior:
  - `tests/agent_cli_tests.rs`
  - `tests/group_commit_tests.rs`
  - `tests/lint_message_cli_tests.rs`

## Contributing Notes

- PRs should target the `develop` branch, not `main`
- Commit messages should follow conventional commit format
- Versioning is managed through `Cargo.toml`, `.committy/config.toml`, and the tag/workflow helpers
