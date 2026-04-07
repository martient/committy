# CLI Workflow Reference

## Core Workflow

```bash
# Commit message authoring
committy commit

# History validation / hook use
committy lint --file .git/COMMIT_EDITMSG --allow-abort --output json

# Release planning
committy bump --dry-run --output json

# Release apply
committy bump --output json

# Standalone changelog
committy changelog --dry-run --output json

# Convention discovery
committy schema --output json
committy example --output json
committy info --output json
committy ls --output json

# Native config scaffold
committy config scaffold --dry-run --output json
```

## Provider Examples

### Cargo

```toml
[release]
provider = "cargo"
tag_format = "v{{ version }}"
```

```bash
committy version --project --output json
committy bump --dry-run --output json
```

### npm

```toml
[release]
provider = "npm"
```

```bash
committy version --project --output json
committy bump --dry-run --output json
```

### Composer

```toml
[release]
provider = "composer"
```

```bash
committy version --project --output json
committy bump --dry-run --output json
```

### PEP 621

```toml
[release]
provider = "pep621"
```

```bash
committy version --project --output json
committy bump --dry-run --output json
```

### Poetry

```toml
[release]
provider = "poetry"
```

```bash
committy version --project --output json
committy bump --dry-run --output json
```

### uv

```toml
[release]
provider = "uv"
```

```bash
committy version --project --output json
committy bump --dry-run --output json
```

### Tag-Backed SCM

```toml
[release]
provider = "scm"
update_files = false
```

```bash
committy version --project --output json
committy bump --dry-run --output json
```

### Multi-Package Repositories

```toml
[release]
provider = "multi-package"
```

```bash
committy version --project --output json
committy bump --dry-run --output json
```

## Custom Convention Example

```toml
[convention]
name = "team-convention"
schema = "<type>(<scope>)!: <description>"
info = "Team-specific conventional commits."

[[convention.types]]
name = "feat"
description = "A feature"
bump = "minor"
changelog_section = "Features"

[[convention.types]]
name = "sec"
description = "A security fix"
bump = "patch"
changelog_section = "Security"

[[convention.questions]]
key = "type"
kind = "select"
prompt = "Select a change type"
required = true
choices = ["feat", "sec", "fix", "docs", "chore"]
```
