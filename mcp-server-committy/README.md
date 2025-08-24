# MCP Server: Committy Wrapper

A Node.js/TypeScript MCP server that wraps the Rust `committy` CLI to provide conventional-commit linting and release tooling via MCP tools.

## Requirements

- Node.js 18+
- Rust `committy` CLI installed and available as `committy` in PATH, or set `COMMITTY_BIN` to its absolute path.

## Install / Build

```bash
# In mcp-server-committy/
npm ci
npm run build
```

## Development

```bash
# Start in watch mode (stdio server)
npm run dev
```

This starts an MCP stdio server and waits for an MCP client to connect.

## Local CLI (npm link)

You can link the package locally to get the CLI binary in your PATH:

```bash
# In mcp-server-committy/
npm link
 
# Now you can run the stdio MCP server directly
mcp-server-committy
```

Note: This is a stdio server intended to be started by an MCP client.

## Run (built)

```bash
# After build
npm start
# or
node dist/index.js
```

If the `committy` binary is not available, set:

```bash
export COMMITTY_BIN=/absolute/path/to/committy
```

## Tools

- mcp0_lint_repo_since_last_tag
  - Input: `{ repo_path: string }`
  - Lints repository commits since the last tag.

- mcp0_lint_message
  - Input: `{ message: string }`
  - Lints a single commit message.

- mcp0_compute_next_tag
  - Input: `{ repo_path: string, fetch?: boolean, prerelease?: boolean, prerelease_suffix?: string, release_branches?: string[] }`
  - Computes the next tag without mutating the repo.

- mcp0_apply_tag
  - Input: `{ repo_path: string, name?: string, fetch?: boolean, prerelease?: boolean, prerelease_suffix?: string, release_branches?: string[], bump_files?: boolean, tag_message?: string, confirm_push: boolean }`
  - Creates/pushes a tag. Refuses unless `confirm_push` is `true`.

- mcp0_format_message
  - Input: `{ commit_type: string, short: string, scope?: string, long?: string, breaking?: boolean }`
  - Returns a conventional commit message from parts.

- mcp0_generate_guidelines
  - Input: `{ repo_path: string, additional_files?: string[], max_bytes?: number }`
  - Reads `README.md`, `CONTRIBUTING.md`, and changelog config if present.

- mcp0_commit_grouped_changes
  - Input: `{ repo_path: string, dry_run?: boolean, include_unstaged?: boolean, auto_stage?: boolean, push?: boolean, signoff?: boolean, confirm?: boolean, group_overrides?: { [group]: { commit_type?: string, scope?: string, short?: string, long?: string } } }`
  - Analyzes pending changes, groups by kind (docs/tests/ci/deps/build/chore/code), returns a plan with suggested conventional commit messages. When `confirm=true` (and `dry_run=false`), stages per group and creates one commit per group. If `push=true`, pushes after committing.

- mcp0_group_commit_plan
  - Input: `{ repo_path: string, include_unstaged?: boolean, ai?: boolean, ai_provider?: "openrouter"|"ollama", ai_model?: string, ai_api_key_env?: string, ai_base_url?: string, ai_max_tokens?: number, ai_temperature?: number, ai_timeout_ms?: number, no_ai_json_mode?: boolean, ai_system_prompt?: string, ai_system_prompt_file?: string, ai_file_limit?: number, ai_allow_sensitive?: boolean }`
  - Wraps Rust `committy group-commit --mode plan --output json`. Returns `{ command: "group-commit", mode: "plan", ok, groups, errors? }`.

- mcp0_group_commit_apply
  - Input: `{ repo_path: string, include_unstaged?: boolean, auto_stage?: boolean, push?: boolean, ai?: boolean, ai_provider?: "openrouter"|"ollama", ai_model?: string, ai_api_key_env?: string, ai_base_url?: string, ai_max_tokens?: number, ai_temperature?: number, ai_timeout_ms?: number, no_ai_json_mode?: boolean, ai_system_prompt?: string, ai_system_prompt_file?: string, ai_file_limit?: number, ai_allow_sensitive?: boolean }`
  - Wraps Rust `committy group-commit --mode apply --output json` and returns `{ command: "group-commit", mode: "apply", ok, groups, commits, pushed?, errors? }`.

## Testing

A simple Node-based test script validates behavior without extra deps:

```bash
npm test
```

This checks:
- `formatMessage()` formatting
- Friendly error when `committy` binary is missing
- `generateGuidelines()` reads common repo files
- `computeNextTag()` and `applyTag()` success flows using a fake CLI script
- `groupCommitPlan()` and `groupCommitApply()` success flows using a fake CLI script

For internal tests, a fake CLI is used by setting:

```bash
export COMMITTY_BIN=node
export COMMITTY_SCRIPT=$(pwd)/scripts/fake-committy.mjs
```

## AI flags & security

- These tools pass AI-related inputs through to the Rust CLI. See `group-commit` flags in the Rust README.
- Key options: `ai`, `ai_provider`, `ai_model`, `ai_api_key_env`, `ai_base_url`, `ai_max_tokens`, `ai_temperature`, `ai_timeout_ms`, `no_ai_json_mode`, `ai_system_prompt`, `ai_system_prompt_file`, `ai_file_limit`, `ai_allow_sensitive`.
- Security: by default, sensitive file contents are not sent. Set `ai_allow_sensitive=true` only if you understand the risks.
- API keys: provide via environment variable named by `ai_api_key_env` on the MCP server process.

## Notes

- Returns tool outputs as `text` (JSON string) to conform with the SDK content types.
- CLI binary name: `mcp-server-committy` (after `npm link`).
