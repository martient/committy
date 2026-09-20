# Agent compatibility & AI capability audit — September 2026

Scope: everything in this repository that an AI coding agent touches — the agent
instruction files, the shared Codex/Claude plugin, the machine-readable CLI
contract (`--output json` / `schema`), the generated enforcement (hooks + CI),
and the in-process LLM integration under `src/ai/`.

Method: full read of the agent-facing surfaces at `caf53e3` (v1.9.1), compared
against the state of the agent-integration ecosystem as of September 2026.

---

## 1. What exists today

| Surface | Location | State |
|---|---|---|
| Agent instructions | `AGENTS.md`, `CLAUDE.md` | Two near-identical files (diff is 2 lines) |
| Claude marketplace | `.claude-plugin/marketplace.json` | Valid, points at `./plugins/committy` |
| Codex marketplace | `.agents/plugins/marketplace.json` | Valid, local source |
| Plugin manifests | `plugins/committy/.claude-plugin/plugin.json`, `.codex-plugin/plugin.json` | Skills only; no commands/agents/hooks/mcpServers |
| Skills | 4 × `SKILL.md` (branch, commit, release, enforce) | Well-scoped, progressive disclosure, safety rules present |
| Codex interface shims | 4 × `agents/openai.yaml` | Display metadata only |
| Machine contract | `src/cli/output.rs` (`api_version = 1`) + per-command payloads | Stable envelope: `api_version`, `command`, `ok`, `dry_run`, `errors` |
| Capability discovery | `src/cli/commands/schema.rs:42-84` | 10 hard-coded capabilities |
| Repo enforcement | `src/cli/commands/hooks.rs:14-37` | `commit-msg`, `pre-push`, GitHub Actions template |
| Local agent guardrail | `.claude/settings.json` + `.claude/hooks/block-dangerous-git.sh` | PreToolUse Bash denylist |
| LLM integration | `src/ai/mod.rs`, `src/cli/commands/group_commit.rs:98-151, 356-507` | OpenRouter + Ollama, `group-commit` only |

The core architectural bet — **CLI-first with typed JSON, not an MCP server** —
is the right one and has been validated by the wider ecosystem this year (see §2).
Nothing below asks you to change that.

---

## 2. What changed in the space (relevant deltas)

1. **`AGENTS.md` is now the cross-tool standard.** Stewarded by the Agentic AI
   Foundation (Linux Foundation), read by 30+ agents (Codex, Copilot, Cursor,
   Gemini CLI, Jules, Aider, Zed, Windsurf, Devin), 60k+ repositories. Claude Code
   remains the outlier reading `CLAUDE.md`; the accepted pattern is a one-line
   `@AGENTS.md` import rather than a maintained duplicate.
2. **Agent Skills has a published spec.** Frontmatter keys are fixed to
   `name`, `description`, `license`, `allowed-tools`, `metadata`, `compatibility`;
   unknown keys fail validation. `allowed-tools` is the security-relevant addition.
3. **`.agents/skills/` is the cross-tool skill discovery path.** Codex CLI, the
   Codex app, Claude Code and Copilot CLI all resolve it. Skills are now the most
   portable plugin component; per-vendor interface files are the least portable.
4. **Codex shipped a plugin marketplace** (26 Mar 2026) bundling skills, MCP
   servers and app integrations; self-serve publishing to the official directory
   is still "coming soon" as of mid-2026.
5. **Claude Code plugins now carry more than skills**: `commands`, `agents`,
   `hooks`, `mcpServers`, `outputStyles`, `lspServers`, auto-discovered from
   standard directories.
6. **CLI beat MCP on token cost for known-interface tools** (roughly 4–32× cheaper;
   a naive GitHub MCP server costs ~55k tokens of context before doing anything).
   The 2026 consensus is hybrid: CLI for tools the model already understands, MCP
   only where you need auth, statefulness, or governance.
7. **Ollama structured output moved to JSON-Schema-constrained decoding** via a
   **top-level** `format` field; `/api/chat` still defaults to `stream: true`.

---

## 3. Findings

Severity: **P0** = broken or misleading in the field · **P1** = real compatibility
or maintenance cost · **P2** = polish.

### 3.1 Defects to fix

**P0-1 — The Ollama provider cannot work as written.**
`src/ai/mod.rs:160-205` posts to `/api/chat` without `stream: false`. Ollama
defaults to `stream: true`, so the response is newline-delimited JSON events, and
`resp.json::<ResponseBody>()` will fail to deserialize on essentially every call.
Every `--ai-provider ollama` run therefore ends in `LlmError::Parse` and silently
falls back to the default message. Fix: send `stream: false`.

**P0-2 — Ollama JSON mode is a no-op.**
`format` is nested inside `options` (`src/ai/mod.rs:174-196`). Ollama reads
`format` as a **top-level** request field, so JSON mode has never taken effect.
Fix: hoist `format` to the request body, and prefer passing the
`AiCommitSuggestion` JSON Schema rather than the string `"json"` — Ollama has
supported schema-constrained decoding since 0.5.

**P0-3 — AI in default (safe) mode is given no signal at all.**
`src/cli/commands/group_commit.rs:400-413`: unless `--ai-allow-sensitive` is set,
the user prompt contains only the group name and the default type/short, and
instructs the model to work "without revealing code or filenames". The model is
asked to improve a description it cannot see the basis for. In practice this
either reproduces the default or produces a lint failure. Fix: send redacted but
real signal by default — path *shapes* (extension histogram, directory prefixes,
add/modify/delete counts, scope mappings from `.committy/config.toml`) — none of
which is file content. Reserve `--ai-allow-sensitive` for actual diff hunks.

**P0-4 — `--ai` is silently ignored in `--mode apply`.**
The AI block lives only in the `"plan"` arm (`group_commit.rs:308-507`); the
`"apply"` arm (`:533+`) rebuilds groups from scratch with no LLM call.
`docs/src/content/docs/reference/ai-flags.mdx` documents
`--mode apply ... --ai --ai-provider ollama` as a working example. Either wire
apply to reuse the planned messages, or reject `--ai` with `--mode apply` and fix
the doc. Silently dropping a flag is the worst of the three options.

**P1-5 — `--ai-diff-lines-per-file` is a dead flag.**
Declared at `group_commit.rs:143-147` as `_ai_diff_lines_per_file` and never read.
It is documented in `ai-flags.mdx` and accepted on the command line. Remove it, or
implement it as part of P0-3.

**P1-6 — `ai-flags.mdx` overstates what is sent and understates what is not.**
It claims `--ai-allow-sensitive` "may include snippets or diff lines from code" —
no diff is ever sent, in either mode. It also recommends
`--ai-model openrouter/anthropic/claude-3.5-sonnet`, a model generation that is two
families out of date. Both should be corrected.

### 3.2 Compatibility gaps to close

**P1-7 — `capabilities` in `schema` is stale and undersells the CLI.**
`schema.rs:42-84` advertises 10 capabilities covering branch, commit and hooks
only. Absent: `release.plan` / `tag.preview` / `tag.apply` / `tag.publish`,
`changelog.preview`, `bump.preview` / `bump.apply`, `group-commit.plan` /
`group-commit.apply`, `packages.*`, `config.validate`, `init`. The
`committy-release` and `committy-enforce` skills already instruct agents to run
`bump`, `tag`, `changelog` and `config validate` — commands the capability list
denies exist. `agent-workflows.mdx` promises capabilities exist precisely "so
skills do not need to hard-code the command surface"; today they must. This is the
single highest-leverage fix in the report: it is the contract every skill reads
first.

Two sub-improvements while you are in there: give each capability a `mutating:
bool` and `requires_confirmation: bool` field so an agent can reason about consent
without pattern-matching command names, and consider deriving the list from the
command registry so it cannot drift again.

**P1-8 — `AGENTS.md` and `CLAUDE.md` are maintained duplicates.**
The diff is two lines. This will drift. Make `AGENTS.md` canonical and reduce
`CLAUDE.md` to an `@AGENTS.md` import plus the Claude-specific skill-invocation
line — the pattern the ecosystem settled on.

**P1-9 — No skills are exposed at `.agents/skills/`.**
`.agents/plugins/marketplace.json` covers Codex plugin installation, but the
cross-tool discovery path that Copilot CLI and others read is `.agents/skills/`.
Exposing the four skills there (symlink, or move the canonical copy there and
point both plugin manifests at it) makes them portable to every agent that
implements the spec, at zero maintenance cost.

**P1-10 — Skill frontmatter omits the spec's optional security fields.**
All four `SKILL.md` files carry only `name` and `description`. The spec now
defines `allowed-tools`, `license`, `metadata` and `compatibility`. Adding
`allowed-tools` to each skill pre-authorizes exactly what the skill needs in
security-conscious runtimes, and `metadata` gives you a version handle for the
skills independent of the CLI version.

**P1-11 — Only two agents are addressed; the instruction file is the cheap part.**
There is no `.github/copilot-instructions.md`, no `GEMINI.md`, no Cursor rules.
Given `AGENTS.md` already exists, these are thin pointer files. Worth doing for
Copilot specifically — it is the widest-deployed agent that does not read
`AGENTS.md` natively.

**P1-12 — No tests cover the agent plugin surface.**
`CLAUDE.md` explicitly asks for "public-seam tests for machine output, exit codes,
Git behavior, hook protocol, and **plugin manifests**." `tests/agent_cli_tests.rs`
covers the first four well (21 tests); nothing validates the marketplace JSON,
plugin manifests, or SKILL.md frontmatter against their schemas. A manifest typo
currently ships silently. Add a test that parses all six JSON/YAML manifests and
asserts every `SKILL.md` frontmatter uses only spec-allowed keys.

**P2-13 — Generated CI template is dated and slow.**
`hooks.rs:31-36` pins `actions/checkout@v4` (v5 is current) and installs via
`cargo install committy --locked` on every PR — a multi-minute Rust build per run
with no caching and no toolchain pin. Prefer downloading the release binary from
`install.sh`, or add `Swatinem/rust-cache`. The same `checkout@v4` pin appears in
all three of this repo's own workflows.

**P2-14 — `$committy-commit` syntax leaks Codex convention into shared skills.**
`committy-release/SKILL.md` closes with "Use `$committy-commit`". Claude Code
invokes these as `/committy:committy-commit` (as `CLAUDE.md` itself documents).
Since the `SKILL.md` body is shared across both runtimes, reference sibling skills
by bare name and let each runtime resolve it.

**P2-15 — The local `block-dangerous-git.sh` guardrail is easily evaded and
over-blocks.** `.claude/hooks/block-dangerous-git.sh` greps the raw command string
for nine substrings. It matches the *text* of a command, not its effect — writing
this very report through a heredoc was blocked because the prose names the push
command. It misses trivial variants (double-space, `env X=1` prefix, `git -C .`),
and it blocks the push-with-upstream form that this repository's own agent
workflow instructions require. It also assumes `jq` is installed with no fallback.
Treat it as a speed bump, not a control: the real enforcement is the `pre-push`
hook, which is policy-engine backed.

### 3.3 Removals

| Remove | Where | Why |
|---|---|---|
| `--ai-diff-lines-per-file` | `group_commit.rs:143-147`, `ai-flags.mdx` | Never read; pure surface area (or implement under P0-3) |
| `LlmError::_Timeout` | `src/ai/mod.rs:12` | Dead variant; timeouts surface as `RequestFailed` |
| Commented-out `LlmProvider` enum | `src/ai/mod.rs:30-35` | Dead code; provider is a `&str` match at the call site |
| Duplicated body of `CLAUDE.md` | `CLAUDE.md` | Replace with `@AGENTS.md` import (P1-8) |
| `agents/openai.yaml` × 4 | `plugins/committy/skills/*/agents/` | Candidate only — verify Codex still reads these; if the marketplace entry supplies display metadata, they are redundant per-vendor files on the least portable layer |
| Empty roadmap page | `docs/src/content/docs/project/roadmap.mdx` | Frontmatter only, no body — publishes an empty page |

### 3.4 Explicitly *not* recommended

- **Do not build a Committy MCP server.** The 2026 evidence is that CLI invocation
  of a tool with a discoverable, typed interface costs a fraction of the tokens an
  MCP tool listing does, and Committy has no auth, statefulness or multi-tenancy
  that would justify the other side of the trade. The `schema` command already
  does MCP's discovery job at a few hundred tokens. Invest in §3.2's P1-7 instead.
- **Do not add an LLM path to the single-commit `commit` command.** The calling
  agent already has the diff in context and writes better messages than a
  256-token side-channel call. Committy's differentiated value is *validating* the
  agent's message, not competing with it.

---

## 4. Suggested order of work

1. **P1-7** — expand and enrich `capabilities` (unblocks every skill; pure win).
2. **P0-1, P0-2** — make Ollama actually function.
3. **P0-4, P1-5, P1-6, §3.3 removals** — stop documenting and accepting things
   that do not happen.
4. **P0-3** — give the default-safe AI path real, non-sensitive signal.
5. **P1-8, P1-9, P1-10** — cross-tool portability (`AGENTS.md` canonical,
   `.agents/skills/`, spec-complete frontmatter).
6. **P1-12** — manifest and frontmatter tests so step 5 cannot regress.
7. **P1-11, P2-13, P2-14, P2-15** — reach, CI cost, consistency, guardrail honesty.

## 5. Sources

- [AGENTS.md Field Guide, 2026 edition](https://www.iuriio.com/blog/posts/2026/05/agents-md-field-guide-2026)
- [AGENTS.md Specification](https://asdlc.io/practices/agents-md-spec/)
- [Agent Skills — Specification](https://agentskills.io/specification)
- [SKILL.md YAML frontmatter reference](https://github.com/shalomb/agent-skills/blob/main/docs/reference/yaml-frontmatter.md)
- [Claude Code plugins reference](https://code.claude.com/docs/en/plugins-reference)
- [Codex CLI plugin marketplace](https://codex.danielvaughan.com/2026/04/24/codex-cli-plugin-marketplace-building-distributing-extending/)
- [OpenAI's Codex gets plugins](https://thenewstack.io/openais-codex-gets-plugins/)
- [Ollama structured outputs](https://docs.ollama.com/capabilities/structured-outputs)
- [Ollama /api/chat reference](https://docs.ollama.com/api/chat)
- [MCP vs CLI for AI agents](https://www.firecrawl.dev/blog/mcp-vs-cli)
- [CLI vs MCP: should your agent call tools or run commands?](https://blog.mcpservers.org/posts/cli-vs-mcp)
