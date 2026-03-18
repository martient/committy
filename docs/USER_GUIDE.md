# Committy User Guide

## Table of Contents

1. [Introduction](#introduction)
2. [Installation](#installation)
3. [Getting Started](#getting-started)
4. [Multi-Package Repository Support](#multi-package-repository-support)
5. [Configuration](#configuration)
6. [Commands](#commands)
7. [Workflows](#workflows)
8. [Best Practices](#best-practices)

## Introduction

Committy is a powerful CLI tool for creating conventional commits with support for multi-package repositories (monorepos). It helps teams maintain consistent commit messages, manage versions across multiple packages, and automate dependency updates.

### Key Features

- 🎯 **Conventional Commits**: Enforces conventional commit format
- 📦 **Multi-Package Support**: Manages Cargo, npm, pnpm, and yarn workspaces
- 🔄 **Version Management**: Independent, unified, or hybrid versioning strategies
- 🎨 **Scope Detection**: Automatic scope detection from changed files
- 🔗 **Dependency Management**: Automatic version updates across files
- 🤖 **AI Integration**: Optional AI-powered commit message generation
- ✅ **Validation**: Built-in linting and validation

## Installation

### From Source

```bash
git clone https://github.com/yourusername/committy.git
cd committy
cargo install --path .
```

### Using Cargo

```bash
cargo install committy
```

## Getting Started

### Single Package Repository

For a simple single-package repository, committy works out of the box:

```bash
# Stage your changes
git add .

# Create a commit
committy commit
```

You'll be prompted to:
1. Select a commit type (feat, fix, chore, etc.)
2. Enter a scope (optional)
3. Write a short description
4. Add a longer description (optional)
5. Mark as breaking change (optional)

### Multi-Package Repository

For monorepos with multiple packages, you'll want to set up configuration:

```bash
# Initialize committy configuration
mkdir -p .committy
cat > .committy/config.toml << 'EOF'
[repository]
name = "my-monorepo"
type = "multi-package"

[versioning]
strategy = "independent"

[[packages]]
name = "cli"
type = "rust-cargo"
path = "packages/cli"

[[packages]]
name = "server"
type = "node-npm"
path = "packages/server"
EOF

# Validate configuration
committy config validate

# List detected packages
committy packages list
```

## Multi-Package Repository Support

### Package Managers

Committy supports multiple package managers:

- **Cargo** (Rust): `Cargo.toml` with workspaces
- **npm** (Node.js): `package.json` with workspaces
- **pnpm** (Node.js): `pnpm-workspace.yaml`
- **yarn** (Node.js): `package.json` with workspaces

### Versioning Strategies

#### Independent Versioning

Each package maintains its own version independently.

```toml
[versioning]
strategy = "independent"

[[packages]]
name = "cli"
type = "rust-cargo"
path = "packages/cli"
independent = true

[[packages]]
name = "server"
type = "node-npm"
path = "packages/server"
independent = true
```

**Use case**: Packages evolve at different rates, have different release cycles.

#### Unified Versioning

All packages share the same version number.

```toml
[versioning]
strategy = "unified"
unified_version = "1.0.0"

[[packages]]
name = "cli"
type = "rust-cargo"
path = "packages/cli"

[[packages]]
name = "server"
type = "node-npm"
path = "packages/server"
```

**Use case**: Tightly coupled packages that should always be released together.

#### Hybrid Versioning

Mix of primary, synced, and independent packages.

```toml
[versioning]
strategy = "hybrid"

[[packages]]
name = "core"
type = "rust-cargo"
path = "packages/core"
primary = true

[[packages]]
name = "cli"
type = "rust-cargo"
path = "packages/cli"
sync_with = "core"

[[packages]]
name = "utils"
type = "rust-cargo"
path = "packages/utils"
independent = true
```

**Use case**: Core packages drive version, some packages sync, others independent.

### Scope Detection

Committy can automatically detect scopes based on changed files:

```toml
[scopes]
auto_detect = true
require_scope_for_multi_package = true
allow_multiple_scopes = true
scope_separator = ","

[[scopes.mappings]]
pattern = "packages/cli/**/*"
scope = "cli"
package = "cli"
description = "CLI application"

[[scopes.mappings]]
pattern = "packages/server/**/*"
scope = "server"
package = "server"
description = "Server application"

[[scopes.mappings]]
pattern = "docs/**/*"
scope = "docs"
package = ""
description = "Documentation"
```

When you commit, committy will:
1. Check which files are staged
2. Match them against patterns
3. Suggest appropriate scopes
4. Allow multiple scopes if changes span packages

### Dependency Management

Automatically update version references across files:

```toml
[[dependencies]]
source = "cli"
description = "CLI version in HELM chart"

[[dependencies.targets]]
file = "deploy/helm/values.yaml"
field = "image.tag"
strategy = "auto"

[[dependencies.targets]]
file = "deploy/docker-compose.yml"
field = "services.cli.image"
strategy = "auto"

[[dependencies]]
source = "server"

[[dependencies.targets]]
file = "Dockerfile"
field = "server"
strategy = "auto"
```

Supported file types:
- **YAML**: HELM charts, configs (dot notation: `image.tag`)
- **JSON**: package.json, configs (dot notation: `dependencies.mypackage`)
- **TOML**: Cargo.toml, configs (dot notation: `dependencies.mylib`)
- **Dockerfile**: FROM/ARG patterns

## Configuration

### Repository Configuration

Create `.committy/config.toml` in your repository root:

```toml
[repository]
name = "my-project"
type = "multi-package"  # or "single-package"
description = "My awesome project"

[versioning]
strategy = "independent"  # or "unified" or "hybrid"
unified_version = "1.0.0"  # only for unified strategy

[versioning.rules]
breaking_change_bumps_major = true
feat_bumps_minor = true
fix_bumps_patch = true

[[packages]]
name = "package-name"
type = "rust-cargo"  # or "node-npm", "node-pnpm", "node-yarn"
path = "path/to/package"
version_file = "Cargo.toml"  # auto-detected
version_field = "package.version"  # auto-detected
primary = false
sync_with = ""  # for hybrid strategy
independent = true
workspace_member = false
description = "Package description"

[scopes]
auto_detect = true
require_scope_for_multi_package = true
allow_multiple_scopes = true
scope_separator = ","

[[scopes.mappings]]
pattern = "path/**/*"
scope = "scope-name"
package = "package-name"
description = "Description"

[[dependencies]]
source = "package-name"
description = "Where this package version is referenced"

[[dependencies.targets]]
file = "path/to/file.yaml"
field = "nested.field.path"
strategy = "auto"  # or "prompt" or "manual"
format = ""  # optional format string

[commit_rules]
max_subject_length = 72
max_body_line_length = 100
require_body = false
allowed_types = []  # empty = all types allowed
custom_types = []

[[commit_rules.custom_types]]
name = "custom"
description = "Custom commit type"
```

### User Configuration

User-level config at `~/.config/committy/config.toml`:

```toml
last_update_check = "2024-01-01T00:00:00Z"
metrics_enabled = true
last_metrics_reminder = "2024-01-01T00:00:00Z"
user_id = "unique-id"

# Regex patterns for version bump detection
major_regex = "BREAKING CHANGE:|!:"
minor_regex = "^feat"
patch_regex = "^fix"
```

## Commands

### commit

Create a new conventional commit.

```bash
# Interactive mode
committy commit

# Non-interactive mode
committy commit --non-interactive \
  --type feat \
  --scope api \
  --message "add user authentication" \
  --body "Implements JWT-based authentication"

# With breaking change
committy commit --breaking

# Skip hooks
committy commit --no-verify
```

### amend

Amend the previous commit.

```bash
committy amend
```

### tag

Create a version tag.

```bash
# Interactive mode
committy tag

# With specific version
committy tag --version 1.2.3

# With message
committy tag --message "Release v1.2.3"

# Dry run
committy tag --dry-run

# JSON output
committy tag --json
```

### lint

Check commits since last tag for conventional format.

```bash
# Lint all commits since last tag
committy lint

# JSON output
committy lint --json

# Verbose output
committy lint --verbose
```

### lint-message

Lint a single commit message.

```bash
# From text
committy lint-message --text "feat: add feature"

# From file
committy lint-message --file commit-msg.txt

# JSON output
committy lint-message --json
```

### config

Manage repository configuration.

```bash
# Show merged configuration
committy config show

# Show as JSON
committy config show --json

# Validate configuration
committy config validate

# Validate with verbose output
committy config validate --verbose
```

### packages

Manage packages in multi-package repositories.

```bash
# List all packages
committy packages list

# List with details
committy packages list --verbose

# Check package status
committy packages status

# Sync package versions
committy packages sync

# Dry run sync
committy packages sync --dry-run
```

### branch

Create a new branch.

```bash
committy branch
```

### group-commit

Group changes and create commits (with optional AI).

```bash
# Plan commits
committy group-commit plan

# Apply planned commits
committy group-commit apply

# With AI
committy group-commit plan --ai
```

## Workflows

### Basic Workflow

```bash
# 1. Make changes
vim src/main.rs

# 2. Stage changes
git add src/main.rs

# 3. Create commit
committy commit

# 4. Push
git push
```

### Multi-Package Workflow

```bash
# 1. Check current package status
committy packages status

# 2. Make changes to multiple packages
vim packages/cli/src/main.rs
vim packages/server/src/index.ts

# 3. Stage changes
git add packages/

# 4. Commit with auto-detected scopes
committy commit
# Committy detects: cli, server

# 5. Check if versions need syncing
committy packages status

# 6. Sync versions if needed
committy packages sync

# 7. Create tags
committy tag

# 8. Push with tags
git push --follow-tags
```

### Release Workflow

```bash
# 1. Ensure clean state
git status
committy packages status

# 2. Lint commits since last release
committy lint

# 3. Create release tag
committy tag --message "Release v1.2.0"

# 4. Verify tag
git tag -l -n9 v1.2.0

# 5. Push release
git push origin main --follow-tags

# 6. Create GitHub release (manual or CI)
```

### CI/CD Integration

```yaml
# .github/workflows/commit-lint.yml
name: Commit Lint

on: [pull_request]

jobs:
  lint:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
        with:
          fetch-depth: 0
      
      - name: Install committy
        run: cargo install committy
      
      - name: Lint commits
        run: committy lint --json
```

## Best Practices

### Commit Messages

✅ **Good:**
```
feat(auth): add JWT authentication

Implements JWT-based authentication with refresh tokens.
Includes middleware for protected routes.

BREAKING CHANGE: Auth header format changed from Basic to Bearer
```

❌ **Bad:**
```
updated stuff
```

### Scopes

- Use **lowercase** scopes
- Keep scopes **short** and **descriptive**
- Use **package names** for multi-package repos
- Use **feature areas** for single-package repos

Examples:
- `auth`, `api`, `ui`, `docs`
- `cli`, `server`, `shared`

### Multi-Package Commits

When changes span multiple packages:

```
feat(cli,server): add health check endpoint

- CLI: Add health check command
- Server: Implement /health endpoint
```

### Version Management

1. **Independent**: Use for loosely coupled packages
2. **Unified**: Use for tightly coupled packages
3. **Hybrid**: Use for mixed coupling scenarios

### Configuration

1. **Commit** `.committy/config.toml` to repository
2. **Don't commit** user config (`~/.config/committy/config.toml`)
3. **Validate** config after changes: `committy config validate`
4. **Document** custom scopes and patterns in README

### Dependency Updates

1. **Define** all version references in config
2. **Use** `auto` strategy for CI/CD
3. **Use** `prompt` strategy for manual review
4. **Test** after dependency updates

## Troubleshooting

### Committy doesn't detect my packages

1. Check package manager files exist:
   - Cargo: `Cargo.toml`
   - npm: `package.json` with `workspaces`
   - pnpm: `pnpm-workspace.yaml`
   - yarn: `package.json` with `workspaces`

2. Verify paths in config:
   ```bash
   committy packages list --verbose
   ```

3. Check max depth (default: 3):
   ```toml
   [repository]
   max_depth = 5
   ```

### Scope detection not working

1. Verify patterns in config:
   ```bash
   committy config show
   ```

2. Check file paths match patterns:
   ```toml
   [[scopes.mappings]]
   pattern = "packages/cli/**/*"  # Matches all files in packages/cli/
   scope = "cli"
   ```

3. Test with staged files:
   ```bash
   git add packages/cli/src/main.rs
   committy commit  # Should suggest "cli" scope
   ```

### Version sync issues

1. Check versioning strategy:
   ```bash
   committy config show | grep strategy
   ```

2. Verify package configuration:
   ```bash
   committy packages status
   ```

3. Manually sync:
   ```bash
   committy packages sync --dry-run  # Preview
   committy packages sync            # Apply
   ```

## Getting Help

- **Documentation**: https://github.com/yourusername/committy/docs
- **Issues**: https://github.com/yourusername/committy/issues
- **Discussions**: https://github.com/yourusername/committy/discussions

## Next Steps

- Read [Configuration Guide](CONFIGURATION.md) for detailed config options
- Check [Examples](examples/) for real-world configurations
- Review [API Documentation](API.md) for programmatic usage
