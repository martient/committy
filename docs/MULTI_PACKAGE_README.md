# Committy - Multi-Package Repository Support

Complete guide to using Committy with multi-package repositories (monorepos).

## Overview

Committy provides comprehensive support for managing multiple packages in a single repository, with features including:

- 📦 **Multi-Package Manager Support**: Cargo, npm, pnpm, yarn
- 🔄 **Flexible Versioning**: Independent, unified, or hybrid strategies
- 🎯 **Automatic Scope Detection**: Based on changed files
- 🔗 **Dependency Management**: Auto-update version references
- ✅ **Package Validation**: Ensure consistency across packages

## Quick Start

### 1. Initialize Configuration

```bash
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

[scopes]
auto_detect = true

[[scopes.mappings]]
pattern = "packages/cli/**/*"
scope = "cli"
package = "cli"

[[scopes.mappings]]
pattern = "packages/server/**/*"
scope = "server"
package = "server"
EOF
```

### 2. Verify Setup

```bash
committy config validate
committy packages list
```

### 3. Make Changes and Commit

```bash
# Edit files
vim packages/cli/src/main.rs
vim packages/server/src/index.ts

# Stage and commit
git add packages/
committy commit
# Scopes auto-detected: cli, server
```

## Features

### Package Detection

Committy automatically detects packages in your repository:

```bash
$ committy packages list --verbose

Found 4 packages:

  cli (rust-cargo)
    Path: packages/cli
    Version: 1.2.0
    File: packages/cli/Cargo.toml
    
  server (node-npm)
    Path: packages/server
    Version: 2.1.0
    File: packages/server/package.json
    
  shared (rust-cargo)
    Path: packages/shared
    Version: 1.0.0
    File: packages/shared/Cargo.toml
    Workspace: true
    
  utils (node-pnpm)
    Path: packages/utils
    Version: 0.5.0
    File: packages/utils/package.json
```

**Supported Package Managers:**

- **Cargo** (Rust): Detects `Cargo.toml` with workspace support
- **npm** (Node.js): Detects `package.json` with workspaces
- **pnpm** (Node.js): Detects `pnpm-workspace.yaml` and `package.json`
- **yarn** (Node.js): Detects `package.json` with workspaces

### Versioning Strategies

#### Independent Versioning

Each package maintains its own version independently.

```toml
[versioning]
strategy = "independent"

[[packages]]
name = "cli"
independent = true
# Version: 1.2.0

[[packages]]
name = "server"
independent = true
# Version: 2.0.1
```

**Best for**: Packages that evolve at different rates.

#### Unified Versioning

All packages share the same version number.

```toml
[versioning]
strategy = "unified"
unified_version = "1.0.0"

[[packages]]
name = "cli"
# Version: 1.0.0

[[packages]]
name = "server"
# Version: 1.0.0
```

**Best for**: Tightly coupled packages that should always be released together.

#### Hybrid Versioning

Mix of primary, synced, and independent packages.

```toml
[versioning]
strategy = "hybrid"

[[packages]]
name = "core"
primary = true
# Drives version: 2.0.0

[[packages]]
name = "cli"
sync_with = "core"
# Syncs with core: 2.0.0

[[packages]]
name = "utils"
independent = true
# Independent: 1.5.0
```

**Best for**: Complex monorepos with mixed coupling.

### Scope Detection

Committy automatically detects scopes based on changed files:

```toml
[scopes]
auto_detect = true
allow_multiple_scopes = true

[[scopes.mappings]]
pattern = "packages/cli/**/*"
scope = "cli"
package = "cli"

[[scopes.mappings]]
pattern = "packages/server/**/*"
scope = "server"
package = "server"
```

**Example:**

```bash
# Edit files in multiple packages
vim packages/cli/src/main.rs
vim packages/server/src/index.ts

# Stage changes
git add packages/

# Commit
committy commit
```

Committy detects:
```
Detected scopes: cli, server
```

Result:
```
feat(cli,server): add health check endpoint

- CLI: Add health check command
- Server: Implement /health endpoint
```

### Dependency Management

Automatically update version references across files:

```toml
[[dependencies]]
source = "cli"
description = "CLI version in HELM chart"

[[dependencies.targets]]
file = "deploy/helm/values.yaml"
field = "cli.image.tag"
strategy = "auto"

[[dependencies.targets]]
file = "Dockerfile"
field = "cli"
strategy = "auto"
```

**Supported File Types:**

- **YAML**: HELM charts, configs (dot notation: `image.tag`)
- **JSON**: package.json, configs (dot notation: `dependencies.pkg`)
- **TOML**: Cargo.toml, configs (dot notation: `dependencies.lib`)
- **Dockerfile**: FROM/ARG patterns

**Example:**

When `cli` version changes from `1.0.0` to `1.1.0`:

```yaml
# deploy/helm/values.yaml (before)
cli:
  image:
    tag: "1.0.0"

# deploy/helm/values.yaml (after)
cli:
  image:
    tag: "1.1.0"  # Auto-updated!
```

### Package Management Commands

#### List Packages

```bash
# Basic list
committy packages list

# With details
committy packages list --verbose

# JSON output
committy packages list --json
```

#### Check Status

```bash
# Check package consistency
committy packages status
```

Output:
```
Package Status:
  cli: v1.2.0 ✓
  server: v2.0.1 ✓
  shared: v1.0.0 ✓
  
All packages are in sync
```

#### Sync Versions

```bash
# Preview changes
committy packages sync --dry-run

# Apply sync
committy packages sync
```

For hybrid strategy, this syncs packages with `sync_with` to their primary package.

## Workflows

### Basic Workflow

```bash
# 1. Check status
committy packages status

# 2. Make changes
vim packages/cli/src/main.rs

# 3. Stage and commit
git add packages/cli/
committy commit

# 4. Verify
committy packages status
```

### Multi-Package Change

```bash
# 1. Make changes to multiple packages
vim packages/cli/src/main.rs
vim packages/server/src/index.ts
vim packages/shared/src/lib.rs

# 2. Stage all changes
git add packages/

# 3. Commit (scopes auto-detected)
committy commit
# Suggests: cli, server, shared

# 4. Check if sync needed
committy packages status

# 5. Sync if using hybrid strategy
committy packages sync
```

### Release Workflow

```bash
# 1. Ensure clean state
git status
committy packages status

# 2. Lint commits
committy lint

# 3. Create tags (for each package or unified)
committy tag

# 4. Push with tags
git push --follow-tags
```

## Configuration Examples

### Rust Workspace

```toml
[repository]
name = "rust-workspace"
type = "multi-package"

[versioning]
strategy = "independent"

[[packages]]
name = "core"
type = "rust-cargo"
path = "crates/core"

[[packages]]
name = "cli"
type = "rust-cargo"
path = "crates/cli"

[[packages]]
name = "server"
type = "rust-cargo"
path = "crates/server"

[scopes]
auto_detect = true

[[scopes.mappings]]
pattern = "crates/core/**/*"
scope = "core"
package = "core"

[[scopes.mappings]]
pattern = "crates/cli/**/*"
scope = "cli"
package = "cli"

[[scopes.mappings]]
pattern = "crates/server/**/*"
scope = "server"
package = "server"
```

### Node.js Monorepo (pnpm)

```toml
[repository]
name = "node-monorepo"
type = "multi-package"

[versioning]
strategy = "unified"
unified_version = "1.0.0"

[[packages]]
name = "@myorg/shared"
type = "node-pnpm"
path = "packages/shared"

[[packages]]
name = "@myorg/web"
type = "node-pnpm"
path = "packages/web"

[[packages]]
name = "@myorg/mobile"
type = "node-pnpm"
path = "packages/mobile"

[scopes]
auto_detect = true

[[scopes.mappings]]
pattern = "packages/shared/**/*"
scope = "shared"
package = "@myorg/shared"

[[scopes.mappings]]
pattern = "packages/web/**/*"
scope = "web"
package = "@myorg/web"

[[scopes.mappings]]
pattern = "packages/mobile/**/*"
scope = "mobile"
package = "@myorg/mobile"
```

### Mixed Language Monorepo

```toml
[repository]
name = "fullstack"
type = "multi-package"

[versioning]
strategy = "hybrid"

[[packages]]
name = "api"
type = "rust-cargo"
path = "backend/api"
primary = true

[[packages]]
name = "worker"
type = "rust-cargo"
path = "backend/worker"
sync_with = "api"

[[packages]]
name = "web"
type = "node-npm"
path = "frontend/web"
sync_with = "api"

[[packages]]
name = "shared"
type = "rust-cargo"
path = "shared"
independent = true

[scopes]
auto_detect = true

[[scopes.mappings]]
pattern = "backend/api/**/*"
scope = "api"
package = "api"

[[scopes.mappings]]
pattern = "backend/worker/**/*"
scope = "worker"
package = "worker"

[[scopes.mappings]]
pattern = "frontend/web/**/*"
scope = "web"
package = "web"

[[scopes.mappings]]
pattern = "shared/**/*"
scope = "shared"
package = "shared"

# Dependency management
[[dependencies]]
source = "api"

[[dependencies.targets]]
file = "deploy/helm/values.yaml"
field = "api.image.tag"
strategy = "auto"

[[dependencies]]
source = "web"

[[dependencies.targets]]
file = "deploy/helm/values.yaml"
field = "web.image.tag"
strategy = "auto"
```

## Best Practices

### 1. Use Descriptive Package Names

```toml
# Good
name = "api-server"
name = "@myorg/shared-utils"

# Avoid
name = "pkg1"
name = "temp"
```

### 2. Group Related Packages

```toml
# Core packages
[[packages]]
name = "core"
path = "packages/core"

[[packages]]
name = "shared"
path = "packages/shared"

# Applications
[[packages]]
name = "cli"
path = "packages/cli"

[[packages]]
name = "server"
path = "packages/server"
```

### 3. Document Scope Mappings

```toml
[[scopes.mappings]]
pattern = "packages/cli/**/*"
scope = "cli"
package = "cli"
description = "CLI application - user-facing command-line tool"
```

### 4. Choose Appropriate Versioning Strategy

- **Independent**: Loosely coupled packages
- **Unified**: Tightly coupled packages
- **Hybrid**: Mixed coupling

### 5. Automate Dependency Updates

```toml
[[dependencies.targets]]
strategy = "auto"  # For CI/CD
# or
strategy = "prompt"  # For manual review
```

### 6. Validate Configuration

```bash
# After editing config
committy config validate
committy packages list
committy config show
```

## Troubleshooting

### Packages Not Detected

```bash
# Check detection
committy packages list --verbose

# Verify package files exist
ls packages/*/Cargo.toml
ls packages/*/package.json

# Check max depth
committy config show | grep max_depth
```

### Scope Not Auto-Detected

```bash
# Check patterns
committy config show

# Test with staged files
git add path/to/file
committy commit
```

### Version Sync Issues

```bash
# Check status
committy packages status

# Preview sync
committy packages sync --dry-run

# Apply sync
committy packages sync
```

## Documentation

- 📖 [User Guide](USER_GUIDE.md) - Complete usage guide
- 🚀 [Quick Start](QUICKSTART.md) - Get started in 5 minutes
- 💡 [Examples](EXAMPLES.md) - Real-world configurations
- 🔧 [Configuration Reference](CONFIGURATION.md) - All configuration options

## Support

- **Issues**: https://github.com/yourusername/committy/issues
- **Discussions**: https://github.com/yourusername/committy/discussions
- **Documentation**: https://github.com/yourusername/committy/docs

---

**Happy committing! 🎉**
