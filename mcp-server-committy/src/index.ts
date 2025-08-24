#!/usr/bin/env node
import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import { z } from "zod";
import {
  lintRepoSinceLastTag,
  lintMessage,
  computeNextTag,
  applyTag,
  formatMessage,
  generateGuidelines,
  groupCommitPlan,
  groupCommitApply,
} from "./committy.js";
import { commitGroupedChanges } from "./commit_groups.js";

const server = new McpServer({ name: "committy", version: "0.1.0" });

// mcp0_lint_repo_since_last_tag
server.registerTool(
  "mcp0_lint_repo_since_last_tag",
  {
    description: "Lint repository commits since last tag using committy CLI",
    inputSchema: {
      repo_path: z.string().describe("Path to the git repository"),
    },
  },
  async ({ repo_path }) => {
    const res = await lintRepoSinceLastTag(repo_path);
    const payload = res.result ?? { stdout: res.raw.stdout, stderr: res.raw.stderr, code: res.raw.code, ok: res.ok };
    return { content: [{ type: "text", text: JSON.stringify(payload) }] };
  }
);

// mcp0_lint_message
server.registerTool(
  "mcp0_lint_message",
  {
    description: "Lint a single commit message for conventional commit compliance",
    inputSchema: {
      message: z.string().describe("Commit message to validate"),
    },
  },
  async ({ message }) => {
    const res = await lintMessage(message);
    const payload = res.result ?? { stdout: res.raw.stdout, stderr: res.raw.stderr, code: res.raw.code, ok: res.ok };
    return { content: [{ type: "text", text: JSON.stringify(payload) }] };
  }
);

// mcp0_compute_next_tag
server.registerTool(
  "mcp0_compute_next_tag",
  {
    description: "Compute the next tag without mutating the repo",
    inputSchema: {
      repo_path: z.string().describe("Path to the git repository"),
      fetch: z.boolean().optional().describe("Whether to git fetch before computing"),
      prerelease: z.boolean().optional().describe("Use prerelease semantics"),
      prerelease_suffix: z.string().optional().describe("Suffix for prerelease, e.g. beta.1"),
      release_branches: z.array(z.string()).optional().describe("Branches considered release branches")
    },
  },
  async ({ repo_path, fetch, prerelease, prerelease_suffix, release_branches }) => {
    const res = await computeNextTag({
      source: repo_path,
      fetch,
      prerelease,
      prereleaseSuffix: prerelease_suffix,
      releaseBranches: release_branches,
    });
    const payload = res.result ?? { stdout: res.raw.stdout, stderr: res.raw.stderr, code: res.raw.code, ok: res.ok };
    return { content: [{ type: "text", text: JSON.stringify(payload) }] };
  }
);

// mcp0_apply_tag
server.registerTool(
  "mcp0_apply_tag",
  {
    description: "Creates and pushes a new tag. Requires confirm_push and allowlist.",
    inputSchema: {
      repo_path: z.string().describe("Path to the git repository"),
      name: z.string().optional().describe("Tag name to create, otherwise computed"),
      fetch: z.boolean().optional(),
      prerelease: z.boolean().optional(),
      prerelease_suffix: z.string().optional(),
      release_branches: z.array(z.string()).optional(),
      bump_files: z.boolean().optional(),
      tag_message: z.string().optional(),
      confirm_push: z.boolean().describe("Must be true to proceed"),
    },
  },
  async ({ repo_path, name, fetch, prerelease, prerelease_suffix, release_branches, bump_files, tag_message, confirm_push }) => {
    if (!confirm_push) {
      return { content: [{ type: "text", text: "confirm_push is false; refusing to mutate repo." }] };
    }
    const res = await applyTag({
      source: repo_path,
      name,
      fetch,
      prerelease,
      prereleaseSuffix: prerelease_suffix,
      releaseBranches: release_branches,
      bumpFiles: bump_files,
      tagMessage: tag_message,
    });
    const payload = res.result ?? { stdout: res.raw.stdout, stderr: res.raw.stderr, code: res.raw.code, ok: res.ok };
    return { content: [{ type: "text", text: JSON.stringify(payload) }] };
  }
);

// mcp0_format_message
server.registerTool(
  "mcp0_format_message",
  {
    description: "Return a conventional commit message from parts",
    inputSchema: {
      commit_type: z.string().describe("Type: feat, fix, docs, chore, refactor, etc."),
      short: z.string().describe("Short description"),
      scope: z.string().optional(),
      long: z.string().optional().describe("Body / long description"),
      breaking: z.boolean().optional(),
    },
  },
  async ({ commit_type, short, scope, long, breaking }) => {
    const text = formatMessage({ commit_type, short, scope, long, breaking });
    return { content: [{ type: "text", text }] };
  }
);

// mcp0_generate_guidelines
server.registerTool(
  "mcp0_generate_guidelines",
  {
    description: "Reads common guideline files (CONTRIBUTING.md, README.md, changelog config) and returns a combined summary.",
    inputSchema: {
      repo_path: z.string().describe("Path to the git repository"),
      additional_files: z.array(z.string()).optional(),
      max_bytes: z.number().optional(),
    },
  },
  async ({ repo_path }) => {
    const result = await generateGuidelines(repo_path);
    return { content: [{ type: "text", text: JSON.stringify(result) }] };
  }
);

// mcp0_commit_grouped_changes
server.registerTool(
  "mcp0_commit_grouped_changes",
  {
    description: "Analyze pending changes, group by kind (docs/tests/ci/deps/build/chore/code), and optionally commit and push.",
    inputSchema: {
      repo_path: z.string().describe("Path to the git repository"),
      dry_run: z.boolean().optional().describe("Only return plan; default true"),
      include_unstaged: z.boolean().optional().describe("Include unstaged changes; default true"),
      auto_stage: z.boolean().optional().describe("Stage files per group before committing; default true"),
      push: z.boolean().optional().describe("Push after committing; default false"),
      signoff: z.boolean().optional().describe("Add Signed-off-by; default false"),
      confirm: z.boolean().optional().describe("Must be true to mutate repo (commit/push)"),
      group_overrides: z
        .record(
          z.object({
            commit_type: z.string().optional(),
            scope: z.string().optional(),
            short: z.string().optional(),
            long: z.string().optional(),
          })
        )
        .optional()
        .describe("Per-group message/type overrides (keys: docs,tests,ci,deps,build,chore,code)"),
    },
  },
  async ({ repo_path, dry_run, include_unstaged, auto_stage, push, signoff, confirm, group_overrides }) => {
    const result = await commitGroupedChanges({
      repoPath: repo_path,
      dryRun: dry_run,
      includeUnstaged: include_unstaged,
      autoStage: auto_stage,
      push,
      signoff,
      confirm,
      groupOverrides: group_overrides as any,
    });
    return { content: [{ type: "text", text: JSON.stringify(result) }] };
  }
);

// mcp0_group_commit_plan (Rust CLI)
server.registerTool(
  "mcp0_group_commit_plan",
  {
    description: "Plan grouped commits using Rust committy CLI (no mutations).",
    inputSchema: {
      repo_path: z.string().describe("Path to the git repository"),
      include_unstaged: z.boolean().optional(),
      // AI flags
      ai: z.boolean().optional(),
      ai_provider: z.enum(["openrouter", "ollama"]).optional(),
      ai_model: z.string().optional(),
      ai_api_key_env: z.string().optional(),
      ai_base_url: z.string().optional(),
      ai_max_tokens: z.number().optional(),
      ai_temperature: z.number().optional(),
      ai_timeout_ms: z.number().optional(),
      no_ai_json_mode: z.boolean().optional(),
      ai_system_prompt: z.string().optional(),
      ai_system_prompt_file: z.string().optional(),
      ai_file_limit: z.number().optional(),
      ai_allow_sensitive: z.boolean().optional(),
    },
  },
  async (input) => {
    const res = await groupCommitPlan(input.repo_path, {
      includeUnstaged: input.include_unstaged,
      ai: input.ai,
      aiProvider: input.ai_provider,
      aiModel: input.ai_model,
      aiApiKeyEnv: input.ai_api_key_env,
      aiBaseUrl: input.ai_base_url,
      aiMaxTokens: input.ai_max_tokens,
      aiTemperature: input.ai_temperature,
      aiTimeoutMs: input.ai_timeout_ms,
      noAiJsonMode: input.no_ai_json_mode,
      aiSystemPrompt: input.ai_system_prompt,
      aiSystemPromptFile: input.ai_system_prompt_file,
      aiFileLimit: input.ai_file_limit,
      aiAllowSensitive: input.ai_allow_sensitive,
    });
    const payload = res.result ?? { stdout: res.raw.stdout, stderr: res.raw.stderr, code: res.raw.code, ok: res.ok };
    return { content: [{ type: "text", text: JSON.stringify(payload) }] };
  }
);

// mcp0_group_commit_apply (Rust CLI)
server.registerTool(
  "mcp0_group_commit_apply",
  {
    description: "Apply grouped commits using Rust committy CLI (mutating).",
    inputSchema: {
      repo_path: z.string().describe("Path to the git repository"),
      include_unstaged: z.boolean().optional(),
      auto_stage: z.boolean().optional(),
      push: z.boolean().optional(),
      // AI flags
      ai: z.boolean().optional(),
      ai_provider: z.enum(["openrouter", "ollama"]).optional(),
      ai_model: z.string().optional(),
      ai_api_key_env: z.string().optional(),
      ai_base_url: z.string().optional(),
      ai_max_tokens: z.number().optional(),
      ai_temperature: z.number().optional(),
      ai_timeout_ms: z.number().optional(),
      no_ai_json_mode: z.boolean().optional(),
      ai_system_prompt: z.string().optional(),
      ai_system_prompt_file: z.string().optional(),
      ai_file_limit: z.number().optional(),
      ai_allow_sensitive: z.boolean().optional(),
    },
  },
  async (input) => {
    const res = await groupCommitApply(input.repo_path, {
      includeUnstaged: input.include_unstaged,
      autoStage: input.auto_stage,
      push: input.push,
      ai: input.ai,
      aiProvider: input.ai_provider,
      aiModel: input.ai_model,
      aiApiKeyEnv: input.ai_api_key_env,
      aiBaseUrl: input.ai_base_url,
      aiMaxTokens: input.ai_max_tokens,
      aiTemperature: input.ai_temperature,
      aiTimeoutMs: input.ai_timeout_ms,
      noAiJsonMode: input.no_ai_json_mode,
      aiSystemPrompt: input.ai_system_prompt,
      aiSystemPromptFile: input.ai_system_prompt_file,
      aiFileLimit: input.ai_file_limit,
      aiAllowSensitive: input.ai_allow_sensitive,
    });
    const payload = res.result ?? { stdout: res.raw.stdout, stderr: res.raw.stderr, code: res.raw.code, ok: res.ok };
    return { content: [{ type: "text", text: JSON.stringify(payload) }] };
  }
);

async function main() {
  const transport = new StdioServerTransport();
  await server.connect(transport);
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
