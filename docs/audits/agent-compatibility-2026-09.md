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
| Capability discovery | `src/cli/commands/schema.rs` | 21 capabilities with consent metadata (was 10, bare) |
| Repo enforcement | `src/cli/commands/hooks.rs:14-37` | `commit-msg`, `pre-push`, GitHub Actions template |
| Local agent guardrail | `.claude/settings.json` + `.claude/hooks/block-dangerous-git.sh` | PreToolUse Bash denylist |
| LLM integration | `src/ai/mod.rs`, `src/cli/commands/group_commit.rs:98-151, 356-507` | OpenRouter + Ollama, `group-commit` only |

The core architectural bet is **CLI-first with typed JSON**. That still holds,
but not for the reason an earlier draft gave: see §3.4, where the blanket
"never build an MCP server" recommendation is withdrawn, and §3.5, where the
CLI's own agent ergonomics turned out to be the real problem.

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
6. **The MCP token argument has a shelf life.** The benchmarks (35× token cost,
   reliability dropping to 72% on hard tasks, ~55k tokens for a naive GitHub
   server) measure *eager-loading* MCP, where every tool schema is injected at
   session start. Progressive disclosure recovers a reported 60–85%.
7. **The 2026-07-28 MCP spec is the largest revision since launch**: stateless
   core, sessions gone, Roots/Sampling/Logging deprecated, MCP Apps and Tasks
   added, progressive discovery and tool search on the roadmap. Recommendations
   written against the old full-bundle design no longer apply cleanly.
8. **Ollama structured output moved to JSON-Schema-constrained decoding** via a
   **top-level** `format` field; `/api/chat` still defaults to `stream: true`.

---

## 3. Findings

Severity: **P0** = broken or misleading in the field · **P1** = real compatibility
or maintenance cost · **P2** = polish.

### 3.1 Defects to fix

**P0-1 — The Ollama provider could not work as written.** ✅ **Fixed.**
`src/ai/mod.rs:160-205` posts to `/api/chat` without `stream: false`. Ollama
defaults to `stream: true`, so the response is newline-delimited JSON events, and
`resp.json::<ResponseBody>()` will fail to deserialize on essentially every call.
Every `--ai-provider ollama` run therefore ends in `LlmError::Parse` and silently
falls back to the default message. Fix: send `stream: false`.

**P0-2 — Ollama JSON mode was a no-op.** ✅ **Fixed.**
`format` is nested inside `options` (`src/ai/mod.rs:174-196`). Ollama reads
`format` as a **top-level** request field, so JSON mode has never taken effect.
Fix: hoist `format` to the request body, and prefer passing the
`AiCommitSuggestion` JSON Schema rather than the string `"json"` — Ollama has
supported schema-constrained decoding since 0.5.

**P0-3 — AI in default (safe) mode was given no signal at all.** ✅ **Fixed.**
`src/cli/commands/group_commit.rs:400-413`: unless `--ai-allow-sensitive` is set,
the user prompt contains only the group name and the default type/short, and
instructs the model to work "without revealing code or filenames". The model is
asked to improve a description it cannot see the basis for. In practice this
either reproduces the default or produces a lint failure. Fix: send redacted but
real signal by default — path *shapes* (extension histogram, directory prefixes,
add/modify/delete counts, scope mappings from `.committy/config.toml`) — none of
which is file content. Reserve `--ai-allow-sensitive` for actual diff hunks.

**P0-4 — WITHDRAWN. This finding was wrong.** ❌
The original text claimed `--ai` was silently ignored in `--mode apply`. It is
not: the apply arm has its own AI block (`group_commit.rs:586`) with its own
`suggest_commit` calls. The error came from reading a `grep` run through
`awk 'NR>=530'`, whose renumbered output was mistaken for line numbers in the
plan arm. The documented `--mode apply ... --ai` example always worked.

What is real in that area is **duplication**: `plan` and `apply` carried
byte-identical copies of both the group-building loop and the ~150-line AI
enrichment block. That duplication is what made the misreading possible. Both
are now extracted into `build_groups()` and `ai_user_prompt()` and shared. ✅ **Fixed.**

**P1-5 — `--ai-diff-lines-per-file` was a dead flag.** ✅ **Fixed** (removed).
Declared at `group_commit.rs:143-147` as `_ai_diff_lines_per_file` and never read.
It is documented in `ai-flags.mdx` and accepted on the command line. Remove it, or
implement it as part of P0-3.

**P1-6 — `ai-flags.mdx` overstated what is sent.** ✅ **Fixed.**
It claims `--ai-allow-sensitive` "may include snippets or diff lines from code" —
no diff is ever sent, in either mode. It also recommends
`--ai-model openrouter/anthropic/claude-3.5-sonnet`, a model generation that is two
families out of date. Both should be corrected.

### 3.2 Compatibility gaps to close

**P1-7 — `capabilities` in `schema` was stale and undersold the CLI.** ✅ **Fixed.**
`schema.rs` advertised 10 capabilities covering branch, commit and hooks only,
while the `committy-release` and `committy-enforce` skills instructed agents to
run `bump`, `tag`, `changelog` and `config validate` — commands the capability
list denied existed. `agent-workflows.mdx` promises capabilities exist precisely
"so skills do not need to hard-code the command surface"; they had to.

Now 21 capabilities cover the whole agent surface, and each declares `mutating`
and `requires_confirmation` so an agent can reason about consent without
pattern-matching command names (`tag.publish` is the only one gated on an
explicit confirmation flag). Covered by
`tests/agent_ergonomics_tests.rs`.

Still open: the list is hand-maintained and can drift again. Deriving it from
the command registry would close that for good.

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

### 3.4 On building an MCP server

An earlier revision of this report said flatly "do not build one". That was
argued against eager-loading MCP, which the protocol has since moved off: the
2026-07-28 spec makes the core stateless, drops sessions, and puts progressive
discovery and tool search on the roadmap. The headline benchmarks against MCP —
35× token cost, reliability falling to 72% on hard tasks — measure the old
full-bundle design; progressive disclosure reportedly recovers 60–85% of that.
So the blanket recommendation was wrong and is withdrawn.

What remains true is the ordering. The observed agent struggle traced to three
mechanical CLI faults (§3.5), not to the CLI *being* a CLI, and all three were
fixable in an afternoon. An MCP server would have fixed them incidentally, by
deleting argv from the problem, while adding a server to build, distribute, and
keep in version lockstep with the binary — and it would not help the Git hook and
CI path at all, which must stay CLI and is where Committy's actual enforcement
lives.

Revisit if agents still struggle now that §3.5 is fixed. If built, shape it as
roughly six stateless tools mapping to capability groups (`policy.discover`,
`commit.preview`/`apply`, `branch.preview`/`apply`, `lint`) rather than eighteen
tools mirroring subcommands, and have `policy.discover` serve the existing
`schema --output json` payload instead of re-describing the surface.

Still not recommended: **an LLM path on the single-commit `commit` command.** The
calling agent already has the diff in context and writes better messages than a
256-token side-channel call. Committy's differentiated value is *validating* the
agent's message, not competing with it.

### 3.5 CLI ergonomics — diagnosed, fixed, and under test

Reproduced against v1.9.1 and now covered by `tests/agent_ergonomics_tests.rs`:

| # | Fault | Fix |
|---|---|---|
| E-1 | `--non-interactive`, `-q` were root-only, so `committy schema --non-interactive …` — the form agents write — was rejected by clap | `global = true` on both |
| E-2 | Argument-parse rejections wrote nothing to stdout and prose to stderr, even with `--output json` | `invalid_usage` envelope on stdout; text mode unchanged |
| E-3 | `COMMITTY_NONINTERACTIVE=1` makes placement irrelevant but appeared in no agent-facing doc | documented in `AGENTS.md`, `CLAUDE.md`, `agent-workflows.mdx` |

`--verbose` was deliberately left root-only: `-v` is `branch --validate` and
`--verbose` belongs to `config validate|show`, so promoting it would silently
steal both shorts. A test pins that trade-off.

---

## 4. Suggested order of work

0. ~~**P1-7**, **E-1**, **E-2**, **E-3**~~ — done: capabilities expanded with
   consent metadata, global flag placement, `invalid_usage` envelope, env var
   documented. See §3.5.
1. ~~**P0-1, P0-2, P0-3**, **P1-5**, **P1-6**~~ — done: Ollama functions,
   the safe AI path carries redacted shape, dead flag and docs corrected.
   **P0-4 was withdrawn as a false finding.**
2. **P1-8, P1-9, P1-10** — cross-tool portability (`AGENTS.md` canonical,
   `.agents/skills/`, spec-complete frontmatter).
3. **P1-12** — manifest and frontmatter tests so the step above cannot regress.
4. **P1-11, P2-13, P2-14, P2-15** — reach, CI cost, consistency, guardrail honesty.

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
- [MCP roadmap: progressive discovery and agent auth](https://www.developersdigest.tech/blog/mcp-roadmap-progressive-discovery-agent-auth)
- [Progressive tool loading is the new MCP context pattern](https://usewire.io/blog/progressive-tool-loading-mcp-context-pattern/)
- [MCP context bloat fix 2026: tool search, code mode, progressive disclosure](https://mcp.directory/blog/mcp-context-bloat-fix-2026-tool-search-code-mode-progressive-disclosure)
- [MCP servers use 35x more tokens than CLI tools](https://www.mindstudio.ai/blog/mcp-servers-35x-more-tokens-cli-tools-reliability-benchmark)
- [MCP vs CLI for AI agents](https://www.firecrawl.dev/blog/mcp-vs-cli)
- [CLI vs MCP: should your agent call tools or run commands?](https://blog.mcpservers.org/posts/cli-vs-mcp)
