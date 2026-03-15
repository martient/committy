# Committy Quick Start Guide

Get up and running with Committy in 5 minutes!

## Installation

```bash
cargo install committy
```

## Single Package Repository

### 1. Create Your First Commit

```bash
# Stage your changes
git add .

# Create a commit
committy commit
```

Follow the prompts:
- **Type**: Select `feat` for new features, `fix` for bug fixes
- **Scope**: (optional) Enter a scope like `api`, `ui`, `auth`
- **Message**: Brief description (e.g., "add user authentication")
- **Body**: (optional) Detailed description
- **Breaking**: (optional) Mark as breaking change

Result:
```
feat(auth): add user authentication

Implements JWT-based authentication with refresh tokens
```

### 2. Create a Version Tag

```bash
committy tag
```

This will:
- Analyze commits since last tag
- Suggest version bump (major/minor/patch)
- Create a new git tag

## Multi-Package Repository (Monorepo)

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
# Validate configuration
committy config validate

# List detected packages
committy packages list
```

Output:
```
Found 2 packages:
  cli (rust-cargo) at packages/cli - v1.0.0
  server (node-npm) at packages/server - v1.2.0
```

### 3. Make Changes and Commit

```bash
# Edit files in multiple packages
vim packages/cli/src/main.rs
vim packages/server/src/index.ts

# Stage changes
git add packages/

# Commit (scopes auto-detected!)
committy commit
```

Committy will detect that you changed both `cli` and `server` and suggest:
```
Detected scopes: cli, server
```

### 4. Check Package Status

```bash
committy packages status
```

Output:
```
Package Status:
  cli: v1.0.0 ✓
  server: v1.2.0 ✓
All packages are in sync
```

### 5. Sync Versions (if needed)

```bash
# Preview changes
committy packages sync --dry-run

# Apply sync
committy packages sync
```

## Common Workflows

### Feature Development

```bash
# 1. Create feature branch
git checkout -b feature/new-feature

# 2. Make changes
vim src/feature.rs

# 3. Commit with committy
git add src/feature.rs
committy commit
# Select: feat
# Scope: feature
# Message: add new feature

# 4. Push
git push origin feature/new-feature
```

### Bug Fix

```bash
# 1. Make fix
vim src/bug.rs

# 2. Commit
git add src/bug.rs
committy commit
# Select: fix
# Scope: bug
# Message: resolve memory leak

# 3. Tag patch version
committy tag
# Suggests: v1.0.1 (patch bump)
```

### Breaking Change

```bash
# 1. Make breaking change
vim src/api.rs

# 2. Commit with breaking flag
git add src/api.rs
committy commit --breaking
# Select: feat
# Message: redesign API

# 3. Tag major version
committy tag
# Suggests: v2.0.0 (major bump)
```

## Configuration Templates

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
```

### Node.js Monorepo (pnpm)

```toml
[repository]
name = "node-monorepo"
type = "multi-package"

[versioning]
strategy = "independent"

[[packages]]
name = "@myorg/shared"
type = "node-pnpm"
path = "packages/shared"

[[packages]]
name = "@myorg/web"
type = "node-pnpm"
path = "packages/web"

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
name = "web"
type = "node-npm"
path = "frontend/web"
sync_with = "api"

[scopes]
auto_detect = true

[[scopes.mappings]]
pattern = "backend/**/*"
scope = "api"
package = "api"

[[scopes.mappings]]
pattern = "frontend/**/*"
scope = "web"
package = "web"
```

## Versioning Strategies

### Independent (Default)

Each package has its own version.

```toml
[versioning]
strategy = "independent"
```

**Best for**: Loosely coupled packages

### Unified

All packages share one version.

```toml
[versioning]
strategy = "unified"
unified_version = "1.0.0"
```

**Best for**: Tightly coupled packages

### Hybrid

Mix of primary, synced, and independent.

```toml
[versioning]
strategy = "hybrid"

[[packages]]
name = "core"
primary = true

[[packages]]
name = "cli"
sync_with = "core"

[[packages]]
name = "utils"
independent = true
```

**Best for**: Complex monorepos

## Tips

### 1. Auto-detect Scopes

Enable automatic scope detection:

```toml
[scopes]
auto_detect = true
```

Committy will suggest scopes based on changed files.

### 2. Multiple Scopes

Allow commits to span multiple packages:

```toml
[scopes]
allow_multiple_scopes = true
scope_separator = ","
```

Result: `feat(cli,server): add feature`

### 3. Validate Before Commit

```bash
# Check what would be committed
committy lint

# Validate configuration
committy config validate
```

### 4. Non-Interactive Mode

For CI/CD:

```bash
committy commit --non-interactive \
  --type feat \
  --scope api \
  --message "add endpoint"
```

### 5. JSON Output

For scripting:

```bash
committy tag --json --dry-run
committy packages list --json
```

## Troubleshooting

### Packages Not Detected

```bash
# Check detection
committy packages list --verbose

# Verify package files exist
ls packages/*/Cargo.toml
ls packages/*/package.json
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

# Force sync
committy packages sync
```

## Next Steps

- 📖 Read the [User Guide](USER_GUIDE.md) for detailed documentation
- 💡 Check [Examples](EXAMPLES.md) for real-world configurations
- 🔧 Review [Configuration Reference](CONFIGURATION.md) for all options
- 🚀 Set up CI/CD integration

## Getting Help

- **Issues**: https://github.com/yourusername/committy/issues
- **Discussions**: https://github.com/yourusername/committy/discussions
- **Documentation**: https://github.com/yourusername/committy/docs

---

**Happy committing! 🎉**
