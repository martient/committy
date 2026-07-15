---
name: committy-enforce
description: Configure and enforce repository commit and branch conventions with Committy Git hooks and CI. Use when an agent is asked to adopt Committy, define typed convention descriptions or branch ticket rules, install hooks, or add convention checks to GitHub Actions.
---

# Enforce with Committy

1. Inspect existing `.committy/config.toml`, hooks, and CI before changing anything.
2. Query `committy -q schema --repo-path <repo> --output json` to understand current typed definitions and supported capabilities.
3. Keep convention types in `[[convention.types]]` with `name`, `description`, `contexts`, bump, changelog, aliases, and visibility fields. Put ticket requirements in `[branch_rules]`.
4. Preview local enforcement with `committy --non-interactive -q hooks install --repo-path <repo> --with-ci --dry-run --output json`.
5. Apply without `--dry-run` after the user authorizes repository writes. Do not use `--force` unless replacement of every reported file is intentional.
6. Verify with `committy --non-interactive -q lint --repo-path <repo> --output json` and a representative branch dry run.

Git hooks and CI are enforcement. Agent instructions are workflow guidance and should stay short.
