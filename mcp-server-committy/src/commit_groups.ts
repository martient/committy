import { formatMessage } from "./committy.js";
import { listChangedFiles, stageFiles, commit as gitCommit, push as gitPush } from "./git.js";

export type ChangeGroupName = "docs" | "tests" | "ci" | "deps" | "build" | "chore" | "code";

export interface GroupOverride {
  commit_type?: string;
  scope?: string;
  short?: string;
  long?: string;
}

export interface GroupPlan {
  name: ChangeGroupName;
  commit_type: string;
  files: string[];
  suggested_message: string;
}

export interface CommitRecord {
  group: ChangeGroupName;
  message: string;
  sha?: string;
  ok: boolean;
  error?: string;
}

export interface CommitGroupedChangesOptions {
  repoPath: string;
  includeUnstaged?: boolean; // default true
  autoStage?: boolean; // default true
  dryRun?: boolean; // default true
  push?: boolean; // default false
  signoff?: boolean; // default false
  confirm?: boolean; // default false - must be true to commit
  groupOverrides?: Partial<Record<ChangeGroupName, GroupOverride>>;
}

export interface CommitGroupedChangesResult {
  ok: boolean;
  groups: GroupPlan[];
  commits?: CommitRecord[];
  pushed?: boolean;
  errors?: string[];
}

export async function commitGroupedChanges(opts: CommitGroupedChangesOptions): Promise<CommitGroupedChangesResult> {
  const includeUnstaged = opts.includeUnstaged ?? true;
  const autoStage = opts.autoStage ?? true;
  const dryRun = opts.dryRun ?? true;
  const confirm = opts.confirm ?? false;

  const { all } = await listChangedFiles(opts.repoPath, includeUnstaged);

  const byGroup: Record<ChangeGroupName, string[]> = {
    docs: [], tests: [], ci: [], deps: [], build: [], chore: [], code: [],
  };

  for (const f of all) {
    byGroup[classifyFile(f)]?.push(f);
  }

  const groups: GroupPlan[] = [];
  (Object.keys(byGroup) as ChangeGroupName[]).forEach((name) => {
    const files = byGroup[name];
    if (!files.length) return;
    const defType = defaultTypeFor(name);
    const override = opts.groupOverrides?.[name];
    const commit_type = override?.commit_type || defType;
    const short = override?.short || defaultShortFor(name);
    const scope = override?.scope;
    const long = override?.long;
    const message = formatMessage({ commit_type, short, scope, long });
    groups.push({ name, commit_type, files, suggested_message: message });
  });

  if (dryRun || !confirm) {
    return { ok: true, groups };
  }

  const commits: CommitRecord[] = [];
  const errors: string[] = [];

  for (const g of groups) {
    if (autoStage) {
      const addRes = await stageFiles(opts.repoPath, g.files);
      if (addRes.code !== 0) {
        const err = `git add failed for group ${g.name}: ${addRes.stderr || addRes.stdout}`;
        errors.push(err);
        commits.push({ group: g.name, message: g.suggested_message, ok: false, error: err });
        continue;
      }
    }
    const c = await gitCommit(opts.repoPath, g.suggested_message, opts.signoff);
    commits.push({ group: g.name, message: g.suggested_message, ok: c.ok, sha: c.sha, error: c.ok ? undefined : (c.raw.stderr || c.raw.stdout) });
    if (!c.ok) {
      errors.push(`commit failed for group ${g.name}: ${c.raw.stderr || c.raw.stdout}`);
    }
  }

  let pushed = false;
  if (opts.push) {
    const pr = await gitPush(opts.repoPath);
    pushed = pr.code === 0;
    if (!pushed) errors.push(`git push failed: ${pr.stderr || pr.stdout}`);
  }

  return { ok: errors.length === 0, groups, commits, pushed, errors: errors.length ? errors : undefined };
}

function defaultTypeFor(name: ChangeGroupName): string {
  switch (name) {
    case "docs": return "docs";
    case "tests": return "test";
    case "ci": return "ci";
    case "deps": return "chore";
    case "build": return "build";
    case "chore": return "chore";
    case "code": return "chore"; // conservative default
  }
}

function defaultShortFor(name: ChangeGroupName): string {
  switch (name) {
    case "docs": return "update docs";
    case "tests": return "update tests";
    case "ci": return "update CI";
    case "deps": return "update dependencies";
    case "build": return "update build config";
    case "chore": return "misc maintenance";
    case "code": return "update code";
  }
}

export function classifyFile(file: string): ChangeGroupName {
  const f = file.replace(/^\.\/?/, "");
  // CI
  if (f.startsWith(".github/")) return "ci";
  // Docs
  if (f.startsWith("docs/") || /(^|\/)README\.md$/i.test(f) || /\.mdx?$/i.test(f)) return "docs";
  // Tests
  if (f.startsWith("tests/") || /\.(test|spec)\.[jt]s$/i.test(f) || /_test\.rs$/i.test(f)) return "tests";
  // Deps (lockfiles)
  if (/(^|\/)package-lock\.json$/.test(f) || /(^|\/)npm-shrinkwrap\.json$/.test(f) || /(^|\/)pnpm-lock\.yaml$/.test(f) || /(^|\/)yarn\.lock$/.test(f) || /(^|\/)Cargo\.lock$/.test(f)) return "deps";
  // Build/config
  if (/(^|\/)Cargo\.toml$/.test(f) || /(^|\/)build\.rs$/.test(f) || /(^|\/)package\.json$/.test(f) || /(^|\/)tsconfig\.json$/.test(f) || /(^|\/)eslint\.(json|js|cjs|yml|yaml|config\.js)$/.test(f) || /(^|\/)\.eslintrc(\..*)?$/.test(f) || /(^|\/)vite\.(config\.)?\w+$/.test(f) || /(^|\/)rollup\.config\.[cm]?js$/.test(f)) return "build";
  // Chore (editor/config meta)
  if (f.startsWith(".vscode/") || /(^|\/)\.editorconfig$/.test(f) || /(^|\/)\.gitignore$/.test(f) || /(^|\/)\.npmrc$/.test(f)) return "chore";
  // Everything else
  return "code";
}
