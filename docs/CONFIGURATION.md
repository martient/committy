# Committy Configuration Reference

Complete reference for all configuration options in Committy.

## Table of Contents

1. [Configuration Files](#configuration-files)
2. [Repository Configuration](#repository-configuration)
3. [User Configuration](#user-configuration)
4. [Configuration Hierarchy](#configuration-hierarchy)
5. [Environment Variables](#environment-variables)

## Configuration Files

### Repository Configuration

**Location**: `.committy/config.toml` (in repository root)

This file is **committed** to the repository and shared across all contributors.

### User Configuration

**Location**: `~/.config/committy/config.toml` (user home directory)

This file is **not committed** and contains user-specific settings.

## Repository Configuration

### `[repository]` Section

Defines repository metadata.

```toml
[repository]
name = "my-project"              # Required: Repository name
type = "multi-package"           # Required: "single-package" or "multi-package"
description = "Description"      # Optional: Repository description
max_depth = 3                    # Optional: Max directory depth for package detection (default: 3)
```

**Fields:**

- `name` (string, required): Repository name
- `type` (string, required): Repository type
  - `"single-package"`: Single package repository
  - `"multi-package"`: Multi-package repository (monorepo)
- `description` (string, optional): Human-readable description
- `max_depth` (integer, optional): Maximum directory depth for package detection (default: 3)

### `[versioning]` Section

Defines versioning strategy and rules.

```toml
[versioning]
strategy = "independent"         # Required: Versioning strategy
unified_version = "1.0.0"        # Optional: For unified strategy

[versioning.rules]
breaking_change_bumps_major = true  # Optional: Default true
feat_bumps_minor = true             # Optional: Default true
fix_bumps_patch = true              # Optional: Default true
```

**Fields:**

- `strategy` (string, required): Versioning strategy
  - `"independent"`: Each package has its own version
  - `"unified"`: All packages share one version
  - `"hybrid"`: Mix of primary, synced, and independent packages
- `unified_version` (string, optional): Version for unified strategy
- `rules` (table, optional): Version bump rules
  - `breaking_change_bumps_major` (boolean): Breaking changes bump major version
  - `feat_bumps_minor` (boolean): Features bump minor version
  - `fix_bumps_patch` (boolean): Fixes bump patch version

### `[[packages]]` Section

Defines packages in the repository. Can have multiple `[[packages]]` entries.

```toml
[[packages]]
name = "package-name"            # Required: Package name
type = "rust-cargo"              # Required: Package type
path = "path/to/package"         # Required: Relative path from repo root
version_file = "Cargo.toml"      # Optional: Auto-detected
version_field = "package.version" # Optional: Auto-detected
primary = false                  # Optional: For hybrid strategy
sync_with = ""                   # Optional: For hybrid strategy
independent = true               # Optional: For hybrid strategy
workspace_member = false         # Optional: Is workspace member
description = "Description"      # Optional: Package description
```

**Fields:**

- `name` (string, required): Package name (must be unique)
- `type` (string, required): Package manager type
  - `"rust-cargo"`: Rust with Cargo
  - `"node-npm"`: Node.js with npm
  - `"node-pnpm"`: Node.js with pnpm
  - `"node-yarn"`: Node.js with yarn
- `path` (string, required): Relative path from repository root
- `version_file` (string, optional): File containing version (auto-detected)
- `version_field` (string, optional): Field path to version (auto-detected)
- `primary` (boolean, optional): Is primary package (hybrid strategy only)
- `sync_with` (string, optional): Sync version with this package (hybrid strategy only)
- `independent` (boolean, optional): Has independent version (hybrid strategy only)
- `workspace_member` (boolean, optional): Is part of a workspace
- `description` (string, optional): Human-readable description

**Package Types:**

| Type | File | Field | Example |
|------|------|-------|---------|
| `rust-cargo` | `Cargo.toml` | `package.version` | `version = "1.0.0"` |
| `node-npm` | `package.json` | `version` | `"version": "1.0.0"` |
| `node-pnpm` | `package.json` | `version` | `"version": "1.0.0"` |
| `node-yarn` | `package.json` | `version` | `"version": "1.0.0"` |

### `[scopes]` Section

Defines scope detection behavior.

```toml
[scopes]
auto_detect = true               # Optional: Enable auto-detection (default: true)
require_scope_for_multi_package = true  # Optional: Require scope in multi-package (default: true)
allow_multiple_scopes = true     # Optional: Allow multiple scopes (default: true)
scope_separator = ","            # Optional: Separator for multiple scopes (default: ",")
```

**Fields:**

- `auto_detect` (boolean, optional): Enable automatic scope detection
- `require_scope_for_multi_package` (boolean, optional): Require scope in multi-package repos
- `allow_multiple_scopes` (boolean, optional): Allow multiple scopes in one commit
- `scope_separator` (string, optional): Separator for multiple scopes (`,`, `/`, `-`, etc.)

### `[[scopes.mappings]]` Section

Defines file pattern to scope mappings. Can have multiple `[[scopes.mappings]]` entries.

```toml
[[scopes.mappings]]
pattern = "path/**/*"            # Required: Glob pattern
scope = "scope-name"             # Required: Scope name
package = "package-name"         # Optional: Associated package
description = "Description"      # Optional: Human-readable description
```

**Fields:**

- `pattern` (string, required): Glob pattern for file matching
  - Supports `**` for recursive matching
  - Supports `*` for single-level matching
  - Examples: `src/**/*.rs`, `packages/cli/**/*`, `*.md`
- `scope` (string, required): Scope name to use when pattern matches
- `package` (string, optional): Associated package name
- `description` (string, optional): Human-readable description

**Pattern Examples:**

```toml
# Match all files in a directory
pattern = "packages/cli/**/*"

# Match specific file types
pattern = "**/*.test.ts"

# Match specific files
pattern = "**/Dockerfile"

# Match top-level directory only
pattern = "src/*"

# Match documentation
pattern = "docs/**/*.md"
```

### `[[dependencies]]` Section

Defines dependency version references. Can have multiple `[[dependencies]]` entries.

```toml
[[dependencies]]
source = "package-name"          # Required: Source package
description = "Description"      # Optional: Human-readable description
```

**Fields:**

- `source` (string, required): Name of the source package whose version is referenced
- `description` (string, optional): Human-readable description

### `[[dependencies.targets]]` Section

Defines where a dependency version is referenced. Each `[[dependencies]]` can have multiple targets.

```toml
[[dependencies.targets]]
file = "path/to/file.yaml"       # Required: File path
field = "nested.field.path"      # Required: Field path (dot notation)
strategy = "auto"                # Required: Update strategy
format = ""                      # Optional: Format string
```

**Fields:**

- `file` (string, required): Relative path to file from repository root
- `field` (string, required): Field path using dot notation
  - YAML/JSON/TOML: `image.tag`, `dependencies.mypackage`
  - Dockerfile: Package name (e.g., `myapp`)
- `strategy` (string, required): Update strategy
  - `"auto"`: Automatically update
  - `"prompt"`: Prompt user for confirmation
  - `"manual"`: Manual update only
- `format` (string, optional): Format string for version (future use)

**File Type Support:**

| File Type | Extension | Field Format | Example |
|-----------|-----------|--------------|---------|
| YAML | `.yaml`, `.yml` | Dot notation | `image.tag` |
| JSON | `.json` | Dot notation | `dependencies.pkg` |
| TOML | `.toml` | Dot notation | `dependencies.lib` |
| Dockerfile | `Dockerfile` | Package name | `myapp` |

**Examples:**

```toml
# YAML file (HELM chart)
[[dependencies.targets]]
file = "deploy/helm/values.yaml"
field = "api.image.tag"
strategy = "auto"

# JSON file (package.json)
[[dependencies.targets]]
file = "packages/web/package.json"
field = "dependencies.@myorg/shared"
strategy = "auto"

# TOML file (Cargo.toml)
[[dependencies.targets]]
file = "crates/cli/Cargo.toml"
field = "dependencies.mylib-core"
strategy = "auto"

# Dockerfile
[[dependencies.targets]]
file = "Dockerfile"
field = "base-image"
strategy = "auto"
```

### `[commit_rules]` Section

Defines commit message rules and validation.

```toml
[commit_rules]
max_subject_length = 72          # Optional: Max subject length (default: 72)
max_body_line_length = 100       # Optional: Max body line length (default: 100)
require_body = false             # Optional: Require body (default: false)
allowed_types = []               # Optional: Allowed commit types (empty = all)
custom_types = []                # Optional: Custom commit types
```

**Fields:**

- `max_subject_length` (integer, optional): Maximum subject line length (default: 72)
- `max_body_line_length` (integer, optional): Maximum body line length (default: 100)
- `require_body` (boolean, optional): Require commit body (default: false)
- `allowed_types` (array, optional): Allowed commit types (empty = all allowed)
- `custom_types` (array, optional): Custom commit type definitions

**Default Commit Types:**

- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation
- `style`: Code style
- `refactor`: Code refactoring
- `perf`: Performance improvement
- `test`: Tests
- `chore`: Maintenance

### `[[commit_rules.custom_types]]` Section

Defines custom commit types.

```toml
[[commit_rules.custom_types]]
name = "custom"                  # Required: Type name
description = "Description"      # Required: Type description
```

**Fields:**

- `name` (string, required): Custom type name (lowercase, no spaces)
- `description` (string, required): Human-readable description

**Example:**

```toml
[[commit_rules.custom_types]]
name = "security"
description = "Security improvements"

[[commit_rules.custom_types]]
name = "deps"
description = "Dependency updates"
```

### `[workspace]` Section

Defines workspace-specific settings (optional).

```toml
[workspace]
root = "."                       # Optional: Workspace root (default: ".")
members = []                     # Optional: Workspace members
```

**Fields:**

- `root` (string, optional): Workspace root directory (default: ".")
- `members` (array, optional): List of workspace member paths

## User Configuration

User-level configuration at `~/.config/committy/config.toml`.

```toml
last_update_check = "2024-01-01T00:00:00Z"  # Last update check timestamp
metrics_enabled = true                       # Enable metrics collection
last_metrics_reminder = "2024-01-01T00:00:00Z"  # Last metrics reminder
user_id = "unique-user-id"                   # Unique user identifier

# Regex patterns for version bump detection
major_regex = "BREAKING CHANGE:|!:"          # Major version bump pattern
minor_regex = "^feat"                        # Minor version bump pattern
patch_regex = "^fix"                         # Patch version bump pattern
```

**Fields:**

- `last_update_check` (datetime): Last time update was checked
- `metrics_enabled` (boolean): Enable anonymous metrics collection
- `last_metrics_reminder` (datetime): Last time metrics reminder was shown
- `user_id` (string): Unique user identifier for metrics
- `major_regex` (string): Regex pattern for major version bumps
- `minor_regex` (string): Regex pattern for minor version bumps
- `patch_regex` (string): Regex pattern for patch version bumps

## Configuration Hierarchy

Configuration is merged in this order (later overrides earlier):

1. **Default values**: Built-in defaults
2. **User configuration**: `~/.config/committy/config.toml`
3. **Repository configuration**: `.committy/config.toml`

### Merge Behavior

- **Scalars** (strings, numbers, booleans): Repository config overrides user config
- **Arrays**: Repository config replaces user config (no merging)
- **Tables**: Merged recursively

**Example:**

User config:
```toml
major_regex = "BREAKING CHANGE:"
minor_regex = "^feat"
```

Repository config:
```toml
major_regex = "!:"
```

Result:
```toml
major_regex = "!:"        # From repository
minor_regex = "^feat"     # From user
```

## Environment Variables

### `COMMITTY_CONFIG_DIR`

Override configuration directory.

```bash
export COMMITTY_CONFIG_DIR=/custom/path
committy commit
```

### `COMMITTY_NO_UPDATE_CHECK`

Disable update checks.

```bash
export COMMITTY_NO_UPDATE_CHECK=1
committy commit
```

### `COMMITTY_NO_METRICS`

Disable metrics collection.

```bash
export COMMITTY_NO_METRICS=1
committy commit
```

### `COMMITTY_LOG_LEVEL`

Set log level.

```bash
export COMMITTY_LOG_LEVEL=debug  # trace, debug, info, warn, error
committy commit
```

## Validation

### Validate Configuration

```bash
# Validate repository config
committy config validate

# Validate with verbose output
committy config validate --verbose

# Show merged configuration
committy config show

# Show as JSON
committy config show --json
```

### Common Validation Errors

**Duplicate package names:**
```
Error: Duplicate package name 'cli' found
```

**Circular dependency:**
```
Error: Circular dependency detected: cli -> server -> cli
```

**Invalid pattern:**
```
Error: Invalid glob pattern in scope mapping: '[invalid'
```

**Missing required field:**
```
Error: Missing required field 'name' in package configuration
```

## Best Practices

### 1. Commit Repository Config

```bash
git add .committy/config.toml
git commit -m "chore: add committy configuration"
```

### 2. Document Custom Scopes

Add comments in config:

```toml
[[scopes.mappings]]
pattern = "packages/cli/**/*"
scope = "cli"
package = "cli"
description = "CLI application - user-facing command-line tool"
```

### 3. Use Descriptive Package Names

```toml
# Good
name = "api-server"
name = "@myorg/shared-utils"

# Avoid
name = "pkg1"
name = "temp"
```

### 4. Group Related Configurations

```toml
# Core packages
[[packages]]
name = "core"
# ...

[[packages]]
name = "shared"
# ...

# Applications
[[packages]]
name = "cli"
# ...

[[packages]]
name = "server"
# ...
```

### 5. Test Configuration Changes

```bash
# After editing config
committy config validate
committy packages list
committy config show
```

## Complete Example

```toml
# .committy/config.toml
[repository]
name = "my-monorepo"
type = "multi-package"
description = "Full-stack application monorepo"
max_depth = 4

[versioning]
strategy = "hybrid"

[versioning.rules]
breaking_change_bumps_major = true
feat_bumps_minor = true
fix_bumps_patch = true

[[packages]]
name = "shared"
type = "rust-cargo"
path = "packages/shared"
primary = true
description = "Shared library"

[[packages]]
name = "cli"
type = "rust-cargo"
path = "packages/cli"
sync_with = "shared"
description = "CLI application"

[[packages]]
name = "server"
type = "node-npm"
path = "packages/server"
sync_with = "shared"
description = "API server"

[[packages]]
name = "utils"
type = "rust-cargo"
path = "packages/utils"
independent = true
description = "Utility functions"

[scopes]
auto_detect = true
require_scope_for_multi_package = true
allow_multiple_scopes = true
scope_separator = ","

[[scopes.mappings]]
pattern = "packages/shared/**/*"
scope = "shared"
package = "shared"
description = "Shared library"

[[scopes.mappings]]
pattern = "packages/cli/**/*"
scope = "cli"
package = "cli"
description = "CLI application"

[[scopes.mappings]]
pattern = "packages/server/**/*"
scope = "server"
package = "server"
description = "API server"

[[scopes.mappings]]
pattern = "packages/utils/**/*"
scope = "utils"
package = "utils"
description = "Utility functions"

[[scopes.mappings]]
pattern = "docs/**/*"
scope = "docs"
package = ""
description = "Documentation"

[[dependencies]]
source = "shared"
description = "Shared library version in dependents"

[[dependencies.targets]]
file = "packages/cli/Cargo.toml"
field = "dependencies.shared"
strategy = "auto"

[[dependencies.targets]]
file = "packages/server/package.json"
field = "dependencies.shared"
strategy = "auto"

[[dependencies]]
source = "server"
description = "Server version in deployment"

[[dependencies.targets]]
file = "deploy/helm/values.yaml"
field = "server.image.tag"
strategy = "auto"

[commit_rules]
max_subject_length = 72
max_body_line_length = 100
require_body = false
allowed_types = ["feat", "fix", "docs", "style", "refactor", "perf", "test", "chore"]

[[commit_rules.custom_types]]
name = "security"
description = "Security improvements"
```

## Next Steps

- Review [User Guide](USER_GUIDE.md) for usage instructions
- Check [Examples](EXAMPLES.md) for real-world configurations
- Read [Quick Start](QUICKSTART.md) for getting started
