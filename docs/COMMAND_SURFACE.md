# Command Surface Overview

Date: 2026-04-07

## Baseline

Committy provides a native command surface for commit authoring, linting, release planning, changelog generation, and convention inspection.

Core commands in this surface:

- `commit`
- `amend`
- `lint`
- `bump`
- `changelog`
- `example`
- `info`
- `ls`
- `schema`
- `version`
- `config scaffold`

## Status

| Capability | Committy status | Notes |
| --- | --- | --- |
| Interactive commit authoring | `has` | `commit` and `amend` remain first-class flows. |
| Convention introspection | `has` | `schema`, `example`, and `info` expose the active Committy-native convention. |
| Release bump orchestration | `has` | `bump` plans or applies version updates, changelog generation, commit, tag, and optional publish. |
| Standalone changelog generation | `has` | `changelog` supports dry-run, range selection, built-in template ids, and custom template paths. |
| Version inspection | `has` | `version` reports the Committy version and the project version from the active provider. |
| Built-in provider coverage | `has` | `cargo`, `npm`, `composer`, `pep621`, `poetry`, `uv`, `scm`, and native `multi-package`. |
| Validation flows | `has` | `lint` covers message, file, rev-range, and abort-friendly hook flows. |
| Convention customization | `has` | `[convention]`, `[[convention.types]]`, and `[[convention.questions]]` provide Committy-native customization. |
| Release customization | `has` | `[release]` controls provider, tag format, bump commit template, tag mode, publish defaults, and hooks. |
| Changelog customization | `has` | `[changelog]` controls template, output file, sections, and inclusion rules. |

## Native Config Surface

Committy’s configuration surface is intentionally native:

- `[convention]` for parser, template, examples, and metadata
- `[[convention.types]]` for change types, aliases, bump impact, and changelog sections
- `[[convention.questions]]` for prompt order and conditional prompting
- `[release]` for providers, tag format, hooks, and publish defaults
- `[changelog]` for template selection, output file, and section mapping

Legacy Committy config still loads:

- `[commit_rules]`
- existing regex-based bump configuration
- existing multi-package `[versioning]` config

Those older fields are normalized into the new convention and release behavior so existing Committy repositories do not break during the upgrade.

## Provider Reference

See [docs/CLI_WORKFLOW_REFERENCE.md](./CLI_WORKFLOW_REFERENCE.md) for verified provider examples covering:

- Cargo
- npm
- Composer
- PEP 621
- Poetry
- uv
- tag-backed SCM
- Committy-managed multi-package repositories
