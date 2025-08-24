#!/usr/bin/env node
// A minimal fake committy CLI to simulate JSON outputs for tests

function getArg(flag) {
  const i = process.argv.indexOf(flag);
  if (i !== -1 && i + 1 < process.argv.length) return process.argv[i + 1];
  return undefined;
}

function has(flag) {
  return process.argv.includes(flag);
}

async function main() {
  const args = process.argv.slice(2);
  // Expected forms:
  // committy --non-interactive lint-message --message <msg> --output json
  // committy --non-interactive tag --output json --source <path> --dry-run (computeNextTag)
  // committy --non-interactive tag --output json --source <path> (applyTag)
  // committy --non-interactive group-commit --mode <plan|apply> --output json [flags]

  if (args.includes('lint-message')) {
    const msg = getArg('--message') || '';
    const out = { command: 'lint-message', valid: true, issues: [], message: msg };
    process.stdout.write(JSON.stringify(out));
    process.exit(0);
  }

  if (args.includes('group-commit')) {
    const mode = getArg('--mode') || 'plan';
    const bad = process.env.FAKE_BAD_JSON;
    const forcedPlanExit = process.env.FAKE_GC_EXIT_PLAN ? parseInt(process.env.FAKE_GC_EXIT_PLAN, 10) : 0;
    const forcedApplyExit = process.env.FAKE_GC_EXIT_APPLY ? parseInt(process.env.FAKE_GC_EXIT_APPLY, 10) : 0;
    const groups = [
      { name: 'docs', commit_type: 'docs', files: ['README.md'], suggested_message: 'docs: update README' },
      { name: 'code', commit_type: 'feat', files: ['src/app.ts'], suggested_message: 'feat: add feature' },
    ];
    if (mode === 'plan') {
      if (bad === 'plan') {
        process.stdout.write('{ not-json: true ');
        process.exit(0);
      }
      const out = { command: 'group-commit', mode: 'plan', ok: true, groups };
      process.stdout.write(JSON.stringify(out));
      process.exit(forcedPlanExit || 0);
    } else {
      if (bad === 'apply') {
        process.stdout.write('not-json');
        process.exit(0);
      }
      const commits = [
        { group: 'docs', message: 'docs: update README', ok: true, sha: 'abc123' },
        { group: 'code', message: 'feat: add feature', ok: true, sha: 'def456' },
      ];
      const pushed = has('--push');
      const out = { command: 'group-commit', mode: 'apply', ok: true, groups, commits, pushed };
      process.stdout.write(JSON.stringify(out));
      process.exit(forcedApplyExit || 0);
    }
  }

  if (args.includes('tag')) {
    const source = getArg('--source') || '';
    if (has('--dry-run')) {
      const out = { command: 'compute-next-tag', source, next_tag: 'v1.2.3' };
      process.stdout.write(JSON.stringify(out));
      process.exit(0);
    }
    const name = getArg('--name') || 'v1.2.3';
    const out = { command: 'apply-tag', source, name, pushed: true };
    process.stdout.write(JSON.stringify(out));
    process.exit(0);
  }

  // Default: unknown command
  process.stderr.write('Unknown command');
  process.exit(1);
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
