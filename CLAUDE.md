# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Overview

Committy is a Rust CLI tool for generating structured, conventional commit messages compatible with SemVer. It supports interactive and non-interactive modes, commit linting, semantic versioning via tags, AI-assisted commit messages, and group commits.

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

# Examples
cargo run -- commit
cargo run -- amend
cargo run -- tag --dry-run
cargo run -- lint --output json

# NEW: Interactive TUI mode
cargo run -- tui
cargo run -- tui --ai  # With AI assistance
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

## Architecture

### Module Structure

**TUI Layer** (`src/tui/`) - NEW!
- `app.rs`: Main TUI application loop and event handling
- `state.rs`: Application state management (files, commits, groups)
- `event.rs`: Keyboard/mouse event handling
- `ui/`: UI components
  - `file_list.rs`: File staging/selection interface
  - `commit_form.rs`: Commit message input form
  - `group_view.rs`: Auto-grouped commits view
  - `help.rs`: Help overlay
- Built with `ratatui` (modern fork of `tui-rs 0.19`)
- Features:
  - Interactive file staging/unstaging
  - Auto-grouping changes by type (docs, tests, ci, deps, code)
  - Multi-commit workflow support
  - Diff preview
  - Real-time UI updates

**CLI Layer** (`src/cli/`)
- `commands/`: Individual command implementations (commit, amend, tag, lint, lint_message, branch, group_commit, tui)
- Each command implements the `Command` trait with `execute(&self, non_interactive: bool)` method
- Commands are defined via `StructOpt` for argument parsing

**Git Operations** (`src/git/`)
- `repository.rs`: Core git operations (staged changes, file listing, config validation)
- `commit.rs`: Commit creation and message formatting
- `tag.rs`: Tag generation with `TagGenerator` and `TagGeneratorOptions`
- `branch.rs`: Branch operations

**Configuration** (`src/config.rs`)
- Config file location: `~/.config/committy/config.toml`
- Override via `COMMITTY_CONFIG_DIR` environment variable
- Contains:
  - `major_regex`, `minor_regex`, `patch_regex`: Configurable regex patterns for semver bump detection
  - `metrics_enabled`: Telemetry toggle
  - `last_update_check`, `last_metrics_reminder`: Timestamps
  - `user_id`: Anonymous UUID for metrics

**Linting** (`src/linter/`)
- Validates conventional commit format
- Used by `lint` and `lint_message` commands
- Returns structured error information

**Version Management** (`src/version/`)
- `VersionManager`: Handles version updates across multiple file types
- Supports Cargo.toml, package.json, pyproject.toml, composer.json, pom.xml, *.csproj
- Called during tag creation to automatically bump version files

**AI Integration** (`src/ai/`)
- Supports OpenRouter and Ollama providers
- Used in `group_commit` command when `--ai` flag is provided
- Generates commit message suggestions based on diffs

**Input/Prompts** (`src/input/`)
- Interactive prompt handling via `inquire` crate
- Input validation for commit messages, scopes, ticket names
- Handles non-interactive mode gracefully

**Error Handling** (`src/error.rs`)
- `CliError`: Central error type for all CLI operations
- Special exit code `3` for lint issues (distinct from general errors)

### Key Flow: Tag Command

1. `TagCommand::execute()` in `src/cli/commands/tag.rs`
2. Creates `TagGenerator` with options (dry-run, fetch, publish)
3. `TagGenerator::generate_and_create_tag()`:
   - Fetches tags from remote (unless `--no-fetch`)
   - Gets latest tag
   - Analyzes commits since last tag using regex patterns from config
   - Determines version bump (major/minor/patch)
   - Updates version files via `VersionManager`
   - Creates git tag (unless `--dry-run`)
   - Publishes tag to remote (unless `--not-publish`)

### Key Flow: Group Commit

1. `GroupCommitCommand::execute()` in `src/cli/commands/group_commit.rs`
2. Groups changed files by type (docs, tests, ci, deps, build, chore, code)
3. In "plan" mode: outputs JSON with suggested commits per group
4. In "apply" mode: creates commits for each group (optionally with AI-generated messages)
5. Optionally pushes commits after creation

### Non-Interactive Mode

- Enabled via `--non-interactive` flag, `COMMITTY_NONINTERACTIVE=1`, or `CI=1`
- All commands support non-interactive mode
- Prompts are skipped; sensible defaults or errors are used

### Build System

- `build.rs`: Injects `SENTRY_DSN` and `POSTHOG_API_KEY` at compile time from environment variables
- Defaults to "undefined" if not set

## Testing

- Tests located in `tests/` directory
- Integration tests use `assert_cmd` for CLI testing
- Many tests require git repositories, use `tempfile` for temporary test repos
- Tests using shared state are marked with `serial_test::serial`

## Contributing Notes

- PRs should target the `develop` branch, not `main`
- Commit messages should follow conventional commit format
- Version is managed in `Cargo.toml` and bumped via the `tag` command