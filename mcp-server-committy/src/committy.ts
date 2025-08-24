import { spawn } from "node:child_process";
import { readFile } from "node:fs/promises";
import path from "node:path";

export type Json = any;

export interface RunResult {
  code: number | null;
  stdout: string;
  stderr: string;
}

export function getCommittyBin(): string {
  return process.env.COMMITTY_BIN || "committy";
}

async function runCommittyRaw(args: string[], opts?: { cwd?: string; env?: NodeJS.ProcessEnv }): Promise<RunResult> {
  return new Promise((resolve) => {
    const script = process.env.COMMITTY_SCRIPT;
    const finalArgs = script ? [script, ...args] : args;
    const child = spawn(getCommittyBin(), finalArgs, {
      cwd: opts?.cwd,
      env: {
        ...process.env,
        CI: "1",
        COMMITTY_NONINTERACTIVE: "1",
        ...(opts?.env || {}),
      },
      stdio: ["ignore", "pipe", "pipe"],
    });

    let stdout = "";
    let stderr = "";

    child.stdout.on("data", (d) => (stdout += d.toString()));
    child.stderr.on("data", (d) => (stderr += d.toString()));

    child.on("error", (err: NodeJS.ErrnoException) => {
      // Common when binary is missing: err.code === 'ENOENT'
      const bin = getCommittyBin();
      const msg = `Failed to spawn committy binary (\"${bin}\"): ${err.code || err.name || "ERROR"}`;
      resolve({ code: 127, stdout, stderr: stderr ? `${stderr}\n${msg}` : msg });
    });

    child.on("close", (code) => resolve({ code, stdout, stderr }));
  });
}

async function runCommittyJson<T = Json>(args: string[], opts?: { cwd?: string; env?: NodeJS.ProcessEnv }): Promise<{ result?: T; raw: RunResult; ok: boolean }> {
  const raw = await runCommittyRaw(args, opts);
  if (raw.stdout.trim().length === 0) {
    return { raw, ok: raw.code === 0 };
  }
  try {
    const parsed = JSON.parse(raw.stdout) as T;
    return { result: parsed, raw, ok: raw.code === 0 };
  } catch (e) {
    return { raw, ok: false };
  }
}

// Lint commits in repo since last tag
export async function lintRepoSinceLastTag(repoPath: string, opts?: { cwd?: string }) {
  const args = ["--non-interactive", "lint", "--repo-path", repoPath, "--output", "json"];
  return runCommittyJson(args, { cwd: opts?.cwd });
}

// Lint a single commit message string
export async function lintMessage(message: string, opts?: { cwd?: string }) {
  const args = [
    "--non-interactive",
    "lint-message",
    "--message",
    message,
    "--output",
    "json",
  ];
  return runCommittyJson(args, { cwd: opts?.cwd });
}

export interface ComputeNextTagOptions {
  source: string; // repo path
  fetch?: boolean; // default: false
  prerelease?: boolean;
  prereleaseSuffix?: string;
  releaseBranches?: string[];
}

export async function computeNextTag(options: ComputeNextTagOptions, opts?: { cwd?: string }) {
  const args = [
    "--non-interactive",
    "tag",
    "--output",
    "json",
    "--source",
    options.source,
    "--dry-run",
  ];
  if (options.fetch === true) args.push("--fetch");
  if (options.fetch === false) args.push("--no-fetch");
  if (options.prerelease) args.push("--prerelease");
  if (options.prereleaseSuffix) args.push("--prerelease-suffix", options.prereleaseSuffix);
  if (options.releaseBranches && options.releaseBranches.length > 0) {
    args.push("--release-branches", options.releaseBranches.join(","));
  }
  return runCommittyJson(args, { cwd: opts?.cwd });
}

export interface ApplyTagOptions {
  source: string; // repo path
  name?: string;
  fetch?: boolean;
  prerelease?: boolean;
  prereleaseSuffix?: string;
  releaseBranches?: string[];
  bumpFiles?: boolean;
  tagMessage?: string;
}

export async function applyTag(options: ApplyTagOptions, opts?: { cwd?: string }) {
  const args = [
    "--non-interactive",
    "tag",
    "--output",
    "json",
    "--source",
    options.source,
  ];
  if (options.name) args.push("--name", options.name);
  if (options.bumpFiles) args.push("--bump-files");
  if (options.tagMessage) args.push("--tag-message", options.tagMessage);
  if (options.fetch) args.push("--fetch");
  if (options.prerelease) args.push("--prerelease");
  if (options.prereleaseSuffix) args.push("--prerelease-suffix", options.prereleaseSuffix);
  if (options.releaseBranches && options.releaseBranches.length > 0) {
    args.push("--release-branches", options.releaseBranches.join(","));
  }
  return runCommittyJson(args, { cwd: opts?.cwd });
}

export interface FormatMessageInput {
  commit_type: string; // feat, fix, chore, docs, refactor, etc.
  short: string; // short description
  scope?: string;
  long?: string; // body
  breaking?: boolean;
}

export function formatMessage(input: FormatMessageInput): string {
  const scope = input.scope ? `(${input.scope})` : "";
  const bang = input.breaking ? "!" : "";
  const header = `${input.commit_type}${scope}${bang}: ${input.short}`;
  const body = input.long ? `\n\n${input.long}\n` : "\n";
  return header + body;
}

export interface GenerateGuidelinesResult {
  readme?: string;
  contributing?: string;
  changelogConfig?: string;
}

export async function generateGuidelines(repoPath: string): Promise<GenerateGuidelinesResult> {
  const tryRead = async (p: string) => {
    try {
      return await readFile(p, "utf8");
    } catch {
      return undefined;
    }
  };
  const candidates = {
    readme: ["README.md", "readme.md"].map((f) => path.join(repoPath, f)),
    contributing: ["CONTRIBUTING.md", ".github/CONTRIBUTING.md"].map((f) => path.join(repoPath, f)),
    changelog: [".github/changelog-config.json", "changelog-config.json", ".github/changelog.json"].map((f) => path.join(repoPath, f)),
  };

  const [readme, contributing, changelogConfig] = await Promise.all([
    (async () => {
      for (const p of candidates.readme) {
        const c = await tryRead(p);
        if (c) return c;
      }
      return undefined;
    })(),
    (async () => {
      for (const p of candidates.contributing) {
        const c = await tryRead(p);
        if (c) return c;
      }
      return undefined;
    })(),
    (async () => {
      for (const p of candidates.changelog) {
        const c = await tryRead(p);
        if (c) return c;
      }
      return undefined;
    })(),
  ]);

  return { readme, contributing, changelogConfig };
}

// -------------------------
// group-commit (Rust CLI)
// -------------------------

export type GroupName = "docs" | "tests" | "ci" | "deps" | "build" | "chore" | "code";

export interface PlanGroup {
  name: GroupName;
  commit_type: string;
  files: string[];
  suggested_message: string;
}

export interface CommitRecord {
  group: GroupName;
  message: string;
  ok: boolean;
  sha?: string;
  error?: string;
}

export interface GroupCommitPlanResult {
  command: "group-commit";
  mode: "plan";
  ok: boolean;
  groups: PlanGroup[];
  errors?: string[];
}

export interface GroupCommitApplyResult {
  command: "group-commit";
  mode: "apply";
  ok: boolean;
  groups: PlanGroup[];
  commits: CommitRecord[];
  pushed?: boolean;
  errors?: string[];
}

export interface GroupCommitCommonOptions {
  includeUnstaged?: boolean;
  // AI options (must match Rust flags)
  ai?: boolean;
  aiProvider?: "openrouter" | "ollama";
  aiModel?: string;
  aiApiKeyEnv?: string;
  aiBaseUrl?: string;
  aiMaxTokens?: number;
  aiTemperature?: number;
  aiTimeoutMs?: number;
  noAiJsonMode?: boolean;
  aiSystemPrompt?: string;
  aiSystemPromptFile?: string;
  aiFileLimit?: number;
  aiAllowSensitive?: boolean;
}

function buildGroupCommitArgs(mode: "plan" | "apply", options?: GroupCommitCommonOptions & { autoStage?: boolean; push?: boolean }): string[] {
  const args: string[] = [
    "--non-interactive",
    "group-commit",
    "--mode",
    mode,
    "--output",
    "json",
  ];
  if (options?.includeUnstaged) args.push("--include-unstaged");
  if (mode === "apply" && options?.autoStage) args.push("--auto-stage");
  if (mode === "apply" && options?.push) args.push("--push");
  if (options?.ai) {
    args.push("--ai");
    if (options.aiProvider) args.push("--ai-provider", options.aiProvider);
    if (options.aiModel) args.push("--ai-model", options.aiModel);
    if (options.aiApiKeyEnv) args.push("--ai-api-key-env", options.aiApiKeyEnv);
    if (options.aiBaseUrl) args.push("--ai-base-url", options.aiBaseUrl);
    if (typeof options.aiMaxTokens === "number") args.push("--ai-max-tokens", String(options.aiMaxTokens));
    if (typeof options.aiTemperature === "number") args.push("--ai-temperature", String(options.aiTemperature));
    if (typeof options.aiTimeoutMs === "number") args.push("--ai-timeout-ms", String(options.aiTimeoutMs));
    if (options.noAiJsonMode) args.push("--no-ai-json-mode");
    if (options.aiSystemPrompt) args.push("--ai-system-prompt", options.aiSystemPrompt);
    if (options.aiSystemPromptFile) args.push("--ai-system-prompt-file", options.aiSystemPromptFile);
    if (typeof options.aiFileLimit === "number") args.push("--ai-file-limit", String(options.aiFileLimit));
    if (options.aiAllowSensitive) args.push("--ai-allow-sensitive");
  }
  return args;
}

export async function groupCommitPlan(repoPath: string, options?: GroupCommitCommonOptions) {
  const args = buildGroupCommitArgs("plan", options);
  return runCommittyJson<GroupCommitPlanResult>(args, { cwd: repoPath });
}

export async function groupCommitApply(
  repoPath: string,
  options?: GroupCommitCommonOptions & { autoStage?: boolean; push?: boolean }
) {
  const args = buildGroupCommitArgs("apply", options);
  return runCommittyJson<GroupCommitApplyResult>(args, { cwd: repoPath });
}
