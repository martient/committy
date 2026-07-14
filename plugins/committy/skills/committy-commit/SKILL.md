---
name: committy-commit
description: Draft, lint, preview, and create repository-compliant commits with Committy. Use when an agent is asked to stage a coherent change, write or repair a commit message, amend a commit, or make a conventional commit.
---

# Commit with Committy

1. Inspect `git status --short` and the relevant diff. Stage only files belonging to the requested change.
2. Query policy once with `committy -q schema --repo-path <repo> --output json`. Choose a commit type from `types`; use `type_definitions` descriptions when intent is ambiguous.
3. Lint a supplied full message with `committy -q lint-message --repo-path <repo> --message <message> --output json`.
4. Preview with `committy --non-interactive -q commit --repo-path <repo> --type <type> --message <description> --dry-run --output json`. Add `--scope`, `--long-message`, or `--breaking-change` only when supported by the change.
5. Inspect `api_version`, `ok`, `message`, `errors`, and optional `workflow`. Correct errors; do not bypass repository policy.
6. Create the commit with the same arguments after the user has authorized committing. Use `amend` only when explicitly requested.
7. Report the SHA and subject. Stop after lint or preview when that is all the user requested.

Never stage unrelated work, publish, tag, or push as part of this skill.
