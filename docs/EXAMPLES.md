# Committy Configuration Examples

This document provides real-world configuration examples for different repository types and use cases.

## Table of Contents

1. [Single Package Examples](#single-package-examples)
2. [Multi-Package Examples](#multi-package-examples)
3. [Versioning Strategy Examples](#versioning-strategy-examples)
4. [Scope Detection Examples](#scope-detection-examples)
5. [Dependency Management Examples](#dependency-management-examples)
6. [Complete Real-World Examples](#complete-real-world-examples)

## Single Package Examples

### Basic Rust Project

```toml
# .committy/config.toml
[repository]
name = "my-rust-app"
type = "single-package"
description = "A Rust CLI application"

[versioning]
strategy = "independent"

[[packages]]
name = "my-rust-app"
type = "rust-cargo"
path = "."
version_file = "Cargo.toml"
version_field = "package.version"

[scopes]
auto_detect = false

[commit_rules]
max_subject_length = 72
max_body_line_length = 100
```

### Basic Node.js Project

```toml
# .committy/config.toml
[repository]
name = "my-node-app"
type = "single-package"

[versioning]
strategy = "independent"

[[packages]]
name = "my-node-app"
type = "node-npm"
path = "."

[scopes]
auto_detect = true

[[scopes.mappings]]
pattern = "src/api/**/*"
scope = "api"
package = "my-node-app"

[[scopes.mappings]]
pattern = "src/ui/**/*"
scope = "ui"
package = "my-node-app"

[[scopes.mappings]]
pattern = "docs/**/*"
scope = "docs"
package = ""
```

## Multi-Package Examples

### Rust Workspace with Multiple Crates

```toml
# .committy/config.toml
[repository]
name = "rust-workspace"
type = "multi-package"
description = "Rust workspace with multiple crates"

[versioning]
strategy = "independent"

[[packages]]
name = "core"
type = "rust-cargo"
path = "crates/core"
independent = true
description = "Core library"

[[packages]]
name = "cli"
type = "rust-cargo"
path = "crates/cli"
independent = true
description = "CLI application"

[[packages]]
name = "server"
type = "rust-cargo"
path = "crates/server"
independent = true
description = "Server application"

[scopes]
auto_detect = true
require_scope_for_multi_package = true
allow_multiple_scopes = true

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

### Node.js Monorepo with npm Workspaces

```toml
# .committy/config.toml
[repository]
name = "node-monorepo"
type = "multi-package"

[versioning]
strategy = "independent"

[[packages]]
name = "@myorg/shared"
type = "node-npm"
path = "packages/shared"
independent = true

[[packages]]
name = "@myorg/web"
type = "node-npm"
path = "packages/web"
independent = true

[[packages]]
name = "@myorg/mobile"
type = "node-npm"
path = "packages/mobile"
independent = true

[scopes]
auto_detect = true
allow_multiple_scopes = true

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
# .committy/config.toml
[repository]
name = "mixed-monorepo"
type = "multi-package"
description = "Rust backend + Node.js frontend"

[versioning]
strategy = "independent"

[[packages]]
name = "api"
type = "rust-cargo"
path = "backend/api"
independent = true

[[packages]]
name = "worker"
type = "rust-cargo"
path = "backend/worker"
independent = true

[[packages]]
name = "web"
type = "node-npm"
path = "frontend/web"
independent = true

[[packages]]
name = "admin"
type = "node-npm"
path = "frontend/admin"
independent = true

[scopes]
auto_detect = true
allow_multiple_scopes = true

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
pattern = "frontend/admin/**/*"
scope = "admin"
package = "admin"

[[scopes.mappings]]
pattern = "infrastructure/**/*"
scope = "infra"
package = ""
description = "Infrastructure and deployment"
```

## Versioning Strategy Examples

### Independent Versioning

Each package has its own version that evolves independently.

```toml
[versioning]
strategy = "independent"

[[packages]]
name = "utils"
type = "rust-cargo"
path = "packages/utils"
independent = true
# Current version: 1.2.3

[[packages]]
name = "cli"
type = "rust-cargo"
path = "packages/cli"
independent = true
# Current version: 2.0.1

[[packages]]
name = "server"
type = "rust-cargo"
path = "packages/server"
independent = true
# Current version: 1.5.0
```

**Use case**: Packages have different release cycles, evolve at different rates.

### Unified Versioning

All packages share the same version number.

```toml
[versioning]
strategy = "unified"
unified_version = "1.0.0"

[[packages]]
name = "core"
type = "rust-cargo"
path = "packages/core"
# Version: 1.0.0

[[packages]]
name = "cli"
type = "rust-cargo"
path = "packages/cli"
# Version: 1.0.0

[[packages]]
name = "server"
type = "rust-cargo"
path = "packages/server"
# Version: 1.0.0
```

**Use case**: Tightly coupled packages, always released together.

### Hybrid Versioning

Mix of primary, synced, and independent packages.

```toml
[versioning]
strategy = "hybrid"

[[packages]]
name = "core"
type = "rust-cargo"
path = "packages/core"
primary = true
# Drives the version: 2.0.0

[[packages]]
name = "cli"
type = "rust-cargo"
path = "packages/cli"
sync_with = "core"
# Syncs with core: 2.0.0

[[packages]]
name = "server"
type = "rust-cargo"
path = "packages/server"
sync_with = "core"
# Syncs with core: 2.0.0

[[packages]]
name = "utils"
type = "rust-cargo"
path = "packages/utils"
independent = true
# Independent: 1.5.0

[[packages]]
name = "dev-tools"
type = "rust-cargo"
path = "packages/dev-tools"
independent = true
# Independent: 0.3.0
```

**Use case**: Core packages drive version, some packages sync, utilities independent.

## Scope Detection Examples

### Feature-Based Scopes

```toml
[scopes]
auto_detect = true

[[scopes.mappings]]
pattern = "src/auth/**/*"
scope = "auth"
package = "myapp"
description = "Authentication and authorization"

[[scopes.mappings]]
pattern = "src/api/**/*"
scope = "api"
package = "myapp"
description = "API endpoints"

[[scopes.mappings]]
pattern = "src/database/**/*"
scope = "db"
package = "myapp"
description = "Database layer"

[[scopes.mappings]]
pattern = "src/ui/**/*"
scope = "ui"
package = "myapp"
description = "User interface"

[[scopes.mappings]]
pattern = "tests/**/*"
scope = "test"
package = ""
description = "Tests"

[[scopes.mappings]]
pattern = "docs/**/*"
scope = "docs"
package = ""
description = "Documentation"
```

### Package-Based Scopes with Submodules

```toml
[scopes]
auto_detect = true
allow_multiple_scopes = true

# Main packages
[[scopes.mappings]]
pattern = "packages/core/**/*"
scope = "core"
package = "core"

[[scopes.mappings]]
pattern = "packages/cli/**/*"
scope = "cli"
package = "cli"

# Shared resources
[[scopes.mappings]]
pattern = "shared/types/**/*"
scope = "types"
package = ""
description = "Shared type definitions"

[[scopes.mappings]]
pattern = "shared/utils/**/*"
scope = "utils"
package = ""
description = "Shared utilities"

# Infrastructure
[[scopes.mappings]]
pattern = "deploy/**/*"
scope = "deploy"
package = ""
description = "Deployment configurations"

[[scopes.mappings]]
pattern = ".github/**/*"
scope = "ci"
package = ""
description = "CI/CD workflows"
```

### Complex Pattern Matching

```toml
[scopes]
auto_detect = true

# Match specific file types
[[scopes.mappings]]
pattern = "**/*.test.ts"
scope = "test"
package = ""

[[scopes.mappings]]
pattern = "**/*.spec.ts"
scope = "test"
package = ""

# Match configuration files
[[scopes.mappings]]
pattern = "**/tsconfig.json"
scope = "config"
package = ""

[[scopes.mappings]]
pattern = "**/package.json"
scope = "deps"
package = ""

# Match documentation
[[scopes.mappings]]
pattern = "**/*.md"
scope = "docs"
package = ""

# Match specific directories
[[scopes.mappings]]
pattern = "src/components/**/*"
scope = "components"
package = "web"

[[scopes.mappings]]
pattern = "src/hooks/**/*"
scope = "hooks"
package = "web"
```

## Dependency Management Examples

### Kubernetes/HELM Chart Updates

```toml
[[dependencies]]
source = "api"
description = "API version in HELM chart"

[[dependencies.targets]]
file = "deploy/helm/values.yaml"
field = "api.image.tag"
strategy = "auto"

[[dependencies.targets]]
file = "deploy/helm/values.yaml"
field = "api.version"
strategy = "auto"

[[dependencies]]
source = "worker"
description = "Worker version in HELM chart"

[[dependencies.targets]]
file = "deploy/helm/values.yaml"
field = "worker.image.tag"
strategy = "auto"
```

Example `values.yaml`:
```yaml
api:
  image:
    repository: myorg/api
    tag: "1.2.3"  # Auto-updated
  version: "1.2.3"  # Auto-updated

worker:
  image:
    repository: myorg/worker
    tag: "2.0.1"  # Auto-updated
```

### Docker Compose Updates

```toml
[[dependencies]]
source = "web"

[[dependencies.targets]]
file = "docker-compose.yml"
field = "services.web.image"
strategy = "auto"

[[dependencies]]
source = "api"

[[dependencies.targets]]
file = "docker-compose.yml"
field = "services.api.image"
strategy = "auto"
```

Example `docker-compose.yml`:
```yaml
services:
  web:
    image: "myorg/web:1.0.0"  # Auto-updated
    ports:
      - "3000:3000"
  
  api:
    image: "myorg/api:2.1.0"  # Auto-updated
    ports:
      - "8080:8080"
```

### Dockerfile Updates

```toml
[[dependencies]]
source = "base-image"

[[dependencies.targets]]
file = "Dockerfile"
field = "base-image"
strategy = "auto"
```

Example `Dockerfile`:
```dockerfile
ARG BASE_IMAGE_VERSION=1.2.3
FROM myorg/base-image:${BASE_IMAGE_VERSION}

# Or direct FROM
FROM myorg/base-image:1.2.3

COPY . /app
WORKDIR /app
```

### Package.json Dependency Updates

```toml
[[dependencies]]
source = "@myorg/shared"

[[dependencies.targets]]
file = "packages/web/package.json"
field = "dependencies.@myorg/shared"
strategy = "auto"

[[dependencies.targets]]
file = "packages/mobile/package.json"
field = "dependencies.@myorg/shared"
strategy = "auto"
```

### Cargo.toml Dependency Updates

```toml
[[dependencies]]
source = "mylib-core"

[[dependencies.targets]]
file = "crates/cli/Cargo.toml"
field = "dependencies.mylib-core"
strategy = "auto"

[[dependencies.targets]]
file = "crates/server/Cargo.toml"
field = "dependencies.mylib-core"
strategy = "auto"
```

## Complete Real-World Examples

### Example 1: Full-Stack TypeScript Monorepo

```toml
# .committy/config.toml
[repository]
name = "fullstack-app"
type = "multi-package"
description = "Full-stack TypeScript application"

[versioning]
strategy = "hybrid"

[versioning.rules]
breaking_change_bumps_major = true
feat_bumps_minor = true
fix_bumps_patch = true

# Core shared library (primary)
[[packages]]
name = "@myapp/shared"
type = "node-pnpm"
path = "packages/shared"
primary = true
description = "Shared types and utilities"

# Frontend (synced with shared)
[[packages]]
name = "@myapp/web"
type = "node-pnpm"
path = "packages/web"
sync_with = "@myapp/shared"
description = "Web application"

# Backend (synced with shared)
[[packages]]
name = "@myapp/api"
type = "node-pnpm"
path = "packages/api"
sync_with = "@myapp/shared"
description = "API server"

# Mobile (synced with shared)
[[packages]]
name = "@myapp/mobile"
type = "node-pnpm"
path = "packages/mobile"
sync_with = "@myapp/shared"
description = "Mobile application"

# Dev tools (independent)
[[packages]]
name = "@myapp/dev-tools"
type = "node-pnpm"
path = "packages/dev-tools"
independent = true
description = "Development tools"

[scopes]
auto_detect = true
require_scope_for_multi_package = true
allow_multiple_scopes = true
scope_separator = ","

[[scopes.mappings]]
pattern = "packages/shared/**/*"
scope = "shared"
package = "@myapp/shared"

[[scopes.mappings]]
pattern = "packages/web/**/*"
scope = "web"
package = "@myapp/web"

[[scopes.mappings]]
pattern = "packages/api/**/*"
scope = "api"
package = "@myapp/api"

[[scopes.mappings]]
pattern = "packages/mobile/**/*"
scope = "mobile"
package = "@myapp/mobile"

[[scopes.mappings]]
pattern = "packages/dev-tools/**/*"
scope = "dev"
package = "@myapp/dev-tools"

[[scopes.mappings]]
pattern = "docs/**/*"
scope = "docs"
package = ""

[[scopes.mappings]]
pattern = ".github/**/*"
scope = "ci"
package = ""

# Dependency management
[[dependencies]]
source = "@myapp/shared"
description = "Shared library version in dependents"

[[dependencies.targets]]
file = "packages/web/package.json"
field = "dependencies.@myapp/shared"
strategy = "auto"

[[dependencies.targets]]
file = "packages/api/package.json"
field = "dependencies.@myapp/shared"
strategy = "auto"

[[dependencies.targets]]
file = "packages/mobile/package.json"
field = "dependencies.@myapp/shared"
strategy = "auto"

[[dependencies]]
source = "@myapp/api"
description = "API version in deployment"

[[dependencies.targets]]
file = "deploy/k8s/values.yaml"
field = "api.image.tag"
strategy = "auto"

[[dependencies]]
source = "@myapp/web"
description = "Web version in deployment"

[[dependencies.targets]]
file = "deploy/k8s/values.yaml"
field = "web.image.tag"
strategy = "auto"

[commit_rules]
max_subject_length = 72
max_body_line_length = 100
require_body = false
```

### Example 2: Rust CLI Tool with Plugins

```toml
# .committy/config.toml
[repository]
name = "rust-cli-tool"
type = "multi-package"
description = "Rust CLI tool with plugin system"

[versioning]
strategy = "hybrid"

# Core CLI (primary)
[[packages]]
name = "mycli"
type = "rust-cargo"
path = "crates/mycli"
primary = true
description = "Main CLI application"

# Core library (synced)
[[packages]]
name = "mycli-core"
type = "rust-cargo"
path = "crates/core"
sync_with = "mycli"
description = "Core library"

# Plugin API (synced)
[[packages]]
name = "mycli-plugin-api"
type = "rust-cargo"
path = "crates/plugin-api"
sync_with = "mycli"
description = "Plugin API"

# Plugins (independent)
[[packages]]
name = "mycli-plugin-git"
type = "rust-cargo"
path = "plugins/git"
independent = true
description = "Git integration plugin"

[[packages]]
name = "mycli-plugin-docker"
type = "rust-cargo"
path = "plugins/docker"
independent = true
description = "Docker integration plugin"

[scopes]
auto_detect = true
allow_multiple_scopes = true

[[scopes.mappings]]
pattern = "crates/mycli/**/*"
scope = "cli"
package = "mycli"

[[scopes.mappings]]
pattern = "crates/core/**/*"
scope = "core"
package = "mycli-core"

[[scopes.mappings]]
pattern = "crates/plugin-api/**/*"
scope = "plugin-api"
package = "mycli-plugin-api"

[[scopes.mappings]]
pattern = "plugins/git/**/*"
scope = "plugin-git"
package = "mycli-plugin-git"

[[scopes.mappings]]
pattern = "plugins/docker/**/*"
scope = "plugin-docker"
package = "mycli-plugin-docker"

# Dependency management
[[dependencies]]
source = "mycli-core"

[[dependencies.targets]]
file = "crates/mycli/Cargo.toml"
field = "dependencies.mycli-core"
strategy = "auto"

[[dependencies]]
source = "mycli-plugin-api"

[[dependencies.targets]]
file = "plugins/git/Cargo.toml"
field = "dependencies.mycli-plugin-api"
strategy = "auto"

[[dependencies.targets]]
file = "plugins/docker/Cargo.toml"
field = "dependencies.mycli-plugin-api"
strategy = "auto"

[commit_rules]
max_subject_length = 72
allowed_types = ["feat", "fix", "docs", "style", "refactor", "perf", "test", "chore"]
```

### Example 3: Microservices with Shared Libraries

```toml
# .committy/config.toml
[repository]
name = "microservices"
type = "multi-package"

[versioning]
strategy = "independent"

# Shared libraries
[[packages]]
name = "common"
type = "rust-cargo"
path = "libs/common"
independent = true

[[packages]]
name = "proto"
type = "rust-cargo"
path = "libs/proto"
independent = true

# Services
[[packages]]
name = "auth-service"
type = "rust-cargo"
path = "services/auth"
independent = true

[[packages]]
name = "user-service"
type = "rust-cargo"
path = "services/user"
independent = true

[[packages]]
name = "payment-service"
type = "rust-cargo"
path = "services/payment"
independent = true

# API Gateway
[[packages]]
name = "api-gateway"
type = "node-npm"
path = "gateway"
independent = true

[scopes]
auto_detect = true
allow_multiple_scopes = true

[[scopes.mappings]]
pattern = "libs/common/**/*"
scope = "common"
package = "common"

[[scopes.mappings]]
pattern = "libs/proto/**/*"
scope = "proto"
package = "proto"

[[scopes.mappings]]
pattern = "services/auth/**/*"
scope = "auth"
package = "auth-service"

[[scopes.mappings]]
pattern = "services/user/**/*"
scope = "user"
package = "user-service"

[[scopes.mappings]]
pattern = "services/payment/**/*"
scope = "payment"
package = "payment-service"

[[scopes.mappings]]
pattern = "gateway/**/*"
scope = "gateway"
package = "api-gateway"

[[scopes.mappings]]
pattern = "deploy/**/*"
scope = "deploy"
package = ""

# Dependency management for HELM charts
[[dependencies]]
source = "auth-service"

[[dependencies.targets]]
file = "deploy/helm/values.yaml"
field = "services.auth.image.tag"
strategy = "auto"

[[dependencies]]
source = "user-service"

[[dependencies.targets]]
file = "deploy/helm/values.yaml"
field = "services.user.image.tag"
strategy = "auto"

[[dependencies]]
source = "payment-service"

[[dependencies.targets]]
file = "deploy/helm/values.yaml"
field = "services.payment.image.tag"
strategy = "auto"

[[dependencies]]
source = "api-gateway"

[[dependencies.targets]]
file = "deploy/helm/values.yaml"
field = "gateway.image.tag"
strategy = "auto"
```

## Tips and Tricks

### 1. Testing Configuration

```bash
# Validate configuration
committy config validate --verbose

# Show merged configuration
committy config show

# Test scope detection
git add path/to/file
committy commit --dry-run
```

### 2. Gradual Migration

Start with minimal config and add features incrementally:

```toml
# Step 1: Basic setup
[repository]
name = "my-project"
type = "multi-package"

# Step 2: Add packages
[[packages]]
name = "package1"
type = "rust-cargo"
path = "packages/package1"

# Step 3: Add scope detection
[scopes]
auto_detect = true

# Step 4: Add dependency management
[[dependencies]]
source = "package1"
```

### 3. Custom Commit Types

```toml
[commit_rules]
allowed_types = ["feat", "fix", "docs", "chore", "custom"]

[[commit_rules.custom_types]]
name = "custom"
description = "Custom commit type for special cases"
```

### 4. Multiple Scope Separators

```toml
[scopes]
allow_multiple_scopes = true
scope_separator = ","  # or "/" or "-"
```

Results in: `feat(web,api): add feature` or `feat(web/api): add feature`

## Next Steps

- Review [User Guide](USER_GUIDE.md) for detailed usage
- Check [Configuration Reference](CONFIGURATION.md) for all options
- See [API Documentation](API.md) for programmatic usage
