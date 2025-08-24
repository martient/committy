import { spawn } from "node:child_process";

export interface GitRunResult {
  code: number | null;
  stdout: string;
  stderr: string;
}

export function runGit(cwd: string, args: string[]): Promise<GitRunResult> {
  return new Promise((resolve) => {
    const child = spawn("git", args, { cwd, stdio: ["ignore", "pipe", "pipe"] });
    let stdout = "";
    let stderr = "";
    child.stdout.setEncoding("utf8");
    child.stderr.setEncoding("utf8");
    child.stdout.on("data", (d) => (stdout += d));
    child.stderr.on("data", (d) => (stderr += d));
    child.on("close", (code) => resolve({ code, stdout, stderr }));
    child.on("error", (err: any) => resolve({ code: 127, stdout: "", stderr: String(err?.message || err) }));
  });
}

export async function listChangedFiles(cwd: string, includeUnstaged: boolean): Promise<{ staged: string[]; unstaged: string[]; all: string[]; }>{
  const stagedRes = await runGit(cwd, ["diff", "--name-only", "--cached", "-z"]);
  const unstagedRes = await runGit(cwd, ["diff", "--name-only", "-z"]);
  const staged = splitNullList(stagedRes.stdout);
  const unstaged = splitNullList(unstagedRes.stdout);
  const all = Array.from(new Set([ ...staged, ...(includeUnstaged ? unstaged : []) ]));
  return { staged, unstaged, all };
}

function splitNullList(out: string): string[] {
  if (!out) return [];
  // Some git versions may output with trailing null
  return out.split("\u0000").filter(Boolean);
}

export async function stageFiles(cwd: string, files: string[]): Promise<GitRunResult> {
  if (!files.length) return { code: 0, stdout: "", stderr: "" };
  return runGit(cwd, ["add", "--", ...files]);
}

export async function commit(cwd: string, message: string, signoff?: boolean): Promise<{ ok: boolean; sha?: string; raw: GitRunResult }>{
  const args = ["commit", "-m", message];
  if (signoff) args.push("-s");
  const res = await runGit(cwd, args);
  if (res.code !== 0) return { ok: false, raw: res };
  const shaRes = await runGit(cwd, ["rev-parse", "HEAD"]);
  const sha = shaRes.stdout.trim();
  return { ok: true, sha, raw: res };
}

export async function push(cwd: string): Promise<GitRunResult> {
  return runGit(cwd, ["push"]);
}
