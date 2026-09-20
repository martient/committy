#!/bin/bash
# Block genuinely destructive git operations.
#
# Design notes:
#   - Matches the *command*, not the raw tool input. The previous version
#     grepped the whole string, so prose in a heredoc that merely named a
#     command was blocked (including this file's own replacement), while
#     `git  push` with two spaces or `git -C . push` slipped straight through.
#   - Plain `git push` is not destructive and is no longer blocked. The
#     repository's own pre-push hook is the policy-backed control; this hook
#     only refuses operations that destroy history or uncommitted work.
#   - Degrades safely when jq is unavailable.

INPUT=$(cat)

if command -v jq >/dev/null 2>&1; then
  COMMAND=$(printf '%s' "$INPUT" | jq -r '.tool_input.command // empty')
else
  COMMAND=$(printf '%s' "$INPUT" | sed -n 's/.*"command"[[:space:]]*:[[:space:]]*"\(.*\)".*/\1/p')
fi

[ -z "$COMMAND" ] && exit 0

# Drop heredoc bodies, so documentation or test data that names a command is
# not mistaken for an invocation of it. A heredoc body spans lines, so this
# keeps the line that opens it and discards everything after.
SCRUBBED=$(printf '%s' "$COMMAND" | awk '/<</ { print; exit } { print }')

# Only inspect commands that actually invoke git.
printf '%s' "$SCRUBBED" | grep -qE '(^|[;&|[:space:]])git([[:space:]]|$)' || exit 0

deny() {
  echo "BLOCKED: '$COMMAND' looks like $1. The user has prevented you from doing this." >&2
  exit 2
}

# --force rewrites remote history unconditionally; --force-with-lease is guarded.
if printf '%s' "$SCRUBBED" | grep -qE 'push([[:space:]]|$).*(--force([[:space:]]|$)|[[:space:]]-f([[:space:]]|$))'; then
  printf '%s' "$SCRUBBED" | grep -q -- '--force-with-lease' || deny "an unguarded force-push"
fi

printf '%s' "$SCRUBBED" | grep -qE 'reset[[:space:]]+--hard' && deny "a hard reset (discards working-tree changes)"
printf '%s' "$SCRUBBED" | grep -qE 'clean[[:space:]]+-[a-zA-Z]*f' && deny "a forced clean (deletes untracked files)"
printf '%s' "$SCRUBBED" | grep -qE 'branch[[:space:]]+-D([[:space:]]|$)' && deny "a forced branch delete"
printf '%s' "$SCRUBBED" | grep -qE '(checkout|restore)[[:space:]]+(--[[:space:]]+)?\.([[:space:]]|$)' && deny "a bulk discard of working-tree changes"

exit 0
