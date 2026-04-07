# Native Git E2E Verification

This repository includes a Docker-based smoke test for the native git integration paths that matter for signed commits, signed tags, and SSH identity selection.

## What it verifies

- `committy commit` creates a signed commit when git signing is enabled.
- `committy amend` and `committy commit --amend` use the same native git commit flow.
- `committy tag` creates a signed annotated tag when git tag signing is enabled.
- `committy tag --publish --confirm-publish` pushes through normal git and ssh resolution.
- Different repositories can use different SSH identities through normal `~/.ssh/config` host aliases.
- `committy commit --git-config ...` can override signing mode or SSH identity per command without changing the default repo/global Git setup.
- `committy tag --fetch` exercises the native git fetch path.

## How to run it

```bash
./scripts/verify_native_git_e2e.sh
```

The script builds a Docker image, compiles `committy`, starts a local `sshd`, generates:

- one OpenPGP signing key
- two distinct SSH keys
- one SSH signing key plus an allowed signers file
- two distinct git users on the local ssh server

It then runs real `committy` operations against those remotes and fails fast if signing or transport behavior does not work.

## When to use it

Run this before shipping changes that affect:

- commit creation
- amend behavior
- tag creation
- remote fetch and push
- repository path handling
- signing support
- SSH transport behavior
