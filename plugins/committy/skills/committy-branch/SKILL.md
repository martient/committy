---
name: committy-branch
description: Preview, validate, and create repository-compliant Git branches with Committy. Use when an agent is asked to name a branch, start work for a ticket, validate a proposed branch name, or switch to a new branch.
---

# Branch with Committy

1. Query `committy -q schema --repo-path <repo> --output json` and select a type whose `contexts` includes `branch`.
2. Prefer structured input: `committy --non-interactive -q branch --repo-path <repo> --type <type> --ticket <ticket> --subject <subject> --dry-run --output json`. Omit `--ticket` only when repository rules allow it.
3. Inspect `api_version`, `ok`, `branch_name`, and `errors`. If validation fails, repair the inputs; never fall back to raw `git branch` to evade policy.
4. Create the branch by repeating the command without `--dry-run` when the requested operation authorizes it.
5. Use `--name` only when the user supplied a full branch name or exact preservation matters. Repository policy may enforce explicit names.

Do not force-recreate an existing branch unless the user explicitly requests it.
