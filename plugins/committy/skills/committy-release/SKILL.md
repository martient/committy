---
name: committy-release
description: Plan and apply version bumps, changelogs, tags, and prereleases with Committy. Use when an agent is asked to prepare, preview, tag, or publish a release, or inspect the release impact of commit history.
---

# Release with Committy

1. Validate repository setup with `committy -q config validate --repo-path <repo> --output json`.
2. Preview the release using `committy --non-interactive -q bump --repo-path <repo> --dry-run --output json`. Use an explicit increment or prerelease only when requested.
3. For narrower work, preview `changelog --dry-run` or `tag --dry-run --no-fetch` directly.
4. Inspect versions, changed files, tags, changelog, publish state, and errors. Preserve the previewed arguments.
5. Apply only after mutation is authorized. Publishing additionally requires the CLI confirmation flag; never infer remote-mutation consent.
6. Report resulting versions, tags, and publish state.

Use `$committy-commit` for ordinary commits and `$committy-enforce` for hooks or CI setup.
