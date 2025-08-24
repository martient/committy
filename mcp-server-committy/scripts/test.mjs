import assert from 'node:assert/strict';
import { mkdtemp, writeFile, mkdir } from 'node:fs/promises';
import path from 'node:path';
import os from 'node:os';
import { formatMessage, lintMessage, generateGuidelines, computeNextTag, applyTag, groupCommitPlan, groupCommitApply } from '../dist/committy.js';

async function testFormatMessage() {
  const msg = formatMessage({
    commit_type: 'feat',
    short: 'add new API',
    scope: 'core',
    long: 'This introduces a new API.',
    breaking: true,
  });
  const expected = 'feat(core)!: add new API\n\nThis introduces a new API.\n';
  assert.equal(msg, expected, 'formatMessage should build proper conventional commit');
}

async function testNonZeroExitWithJson() {
  const oldBin = process.env.COMMITTY_BIN;
  const oldScript = process.env.COMMITTY_SCRIPT;
  const oldPlanExit = process.env.FAKE_GC_EXIT_PLAN;
  const oldApplyExit = process.env.FAKE_GC_EXIT_APPLY;
  try {
    process.env.COMMITTY_BIN = 'node';
    const dirname = path.dirname(new URL(import.meta.url).pathname);
    process.env.COMMITTY_SCRIPT = path.join(dirname, 'fake-committy.mjs');

    const tmpRepo = await mkdtemp(path.join(os.tmpdir(), 'committy-gc-nonzero-'));

    process.env.FAKE_GC_EXIT_PLAN = '3';
    const plan = await groupCommitPlan(tmpRepo, {});
    assert.equal(plan.ok, false, 'plan ok should be false when exit code is non-zero');
    assert.ok(plan.result && plan.result.mode === 'plan', 'plan should still parse JSON result');

    process.env.FAKE_GC_EXIT_APPLY = '2';
    const apply = await groupCommitApply(tmpRepo, {});
    assert.equal(apply.ok, false, 'apply ok should be false when exit code is non-zero');
    assert.ok(apply.result && apply.result.mode === 'apply', 'apply should still parse JSON result');
  } finally {
    if (oldBin === undefined) delete process.env.COMMITTY_BIN; else process.env.COMMITTY_BIN = oldBin;
    if (oldScript === undefined) delete process.env.COMMITTY_SCRIPT; else process.env.COMMITTY_SCRIPT = oldScript;
    if (oldPlanExit === undefined) delete process.env.FAKE_GC_EXIT_PLAN; else process.env.FAKE_GC_EXIT_PLAN = oldPlanExit;
    if (oldApplyExit === undefined) delete process.env.FAKE_GC_EXIT_APPLY; else process.env.FAKE_GC_EXIT_APPLY = oldApplyExit;
  }
}

async function testBadJsonGroupCommit() {
  const oldBin = process.env.COMMITTY_BIN;
  const oldScript = process.env.COMMITTY_SCRIPT;
  const oldBad = process.env.FAKE_BAD_JSON;
  try {
    process.env.COMMITTY_BIN = 'node';
    const dirname = path.dirname(new URL(import.meta.url).pathname);
    process.env.COMMITTY_SCRIPT = path.join(dirname, 'fake-committy.mjs');

    const tmpRepo = await mkdtemp(path.join(os.tmpdir(), 'committy-gc-bad-'));

    process.env.FAKE_BAD_JSON = 'plan';
    const plan = await groupCommitPlan(tmpRepo, { includeUnstaged: true });
    assert.equal(plan.ok, false, 'plan ok should be false when JSON is malformed');
    assert.equal(plan.result, undefined, 'plan result should be undefined on JSON parse failure');

    process.env.FAKE_BAD_JSON = 'apply';
    const apply = await groupCommitApply(tmpRepo, { autoStage: true });
    assert.equal(apply.ok, false, 'apply ok should be false when JSON is malformed');
    assert.equal(apply.result, undefined, 'apply result should be undefined on JSON parse failure');
  } finally {
    if (oldBin === undefined) delete process.env.COMMITTY_BIN; else process.env.COMMITTY_BIN = oldBin;
    if (oldScript === undefined) delete process.env.COMMITTY_SCRIPT; else process.env.COMMITTY_SCRIPT = oldScript;
    if (oldBad === undefined) delete process.env.FAKE_BAD_JSON; else process.env.FAKE_BAD_JSON = oldBad;
  }
}

async function testFakeCliGroupCommitPlanApply() {
  const oldBin = process.env.COMMITTY_BIN;
  const oldScript = process.env.COMMITTY_SCRIPT;
  try {
    process.env.COMMITTY_BIN = 'node';
    const dirname = path.dirname(new URL(import.meta.url).pathname);
    process.env.COMMITTY_SCRIPT = path.join(dirname, 'fake-committy.mjs');

    const tmpRepo = await mkdtemp(path.join(os.tmpdir(), 'committy-gc-'));

    const plan = await groupCommitPlan(tmpRepo, { includeUnstaged: true, ai: false });
    assert.equal(plan.ok, true, 'groupCommitPlan should succeed with fake CLI');
    assert.ok(plan.result && plan.result.command === 'group-commit', 'plan result has command');
    assert.equal(plan.result.mode, 'plan', 'plan mode is plan');
    assert.ok(Array.isArray(plan.result.groups) && plan.result.groups.length >= 1, 'plan has groups');

    const apply = await groupCommitApply(tmpRepo, { autoStage: true, push: true });
    assert.equal(apply.ok, true, 'groupCommitApply should succeed with fake CLI');
    assert.equal(apply.result?.mode, 'apply', 'apply mode is apply');
    assert.ok(Array.isArray(apply.result?.commits) && apply.result.commits.length >= 1, 'apply has commits');
    assert.equal(apply.result?.pushed, true, 'apply pushed should be true when --push is passed');
  } finally {
    if (oldBin === undefined) delete process.env.COMMITTY_BIN; else process.env.COMMITTY_BIN = oldBin;
    if (oldScript === undefined) delete process.env.COMMITTY_SCRIPT; else process.env.COMMITTY_SCRIPT = oldScript;
  }
}

async function testFakeCliComputeAndApplyTag() {
  const oldBin = process.env.COMMITTY_BIN;
  const oldScript = process.env.COMMITTY_SCRIPT;
  try {
    // Use node to run the fake CLI script
    process.env.COMMITTY_BIN = 'node';
    const dirname = path.dirname(new URL(import.meta.url).pathname);
    process.env.COMMITTY_SCRIPT = path.join(dirname, 'fake-committy.mjs');

    const comp = await computeNextTag({ source: '/tmp/repo', fetch: false });
    assert.equal(comp.ok, true, 'computeNextTag should succeed with fake CLI');
    assert.ok(comp.result && comp.result.next_tag === 'v1.2.3', 'computeNextTag returns next_tag');

    const appl = await applyTag({ source: '/tmp/repo', name: 'v1.2.3' });
    assert.equal(appl.ok, true, 'applyTag should succeed with fake CLI');
    assert.ok(appl.result && appl.result.name === 'v1.2.3', 'applyTag returns name');
  } finally {
    if (oldBin === undefined) delete process.env.COMMITTY_BIN; else process.env.COMMITTY_BIN = oldBin;
    if (oldScript === undefined) delete process.env.COMMITTY_SCRIPT; else process.env.COMMITTY_SCRIPT = oldScript;
  }
}

async function testMissingBinary() {
  const old = process.env.COMMITTY_BIN;
  try {
    process.env.COMMITTY_BIN = '/nonexistent/committy-binary';
    const res = await lintMessage('feat: test');
    assert.equal(res.ok, false, 'ok should be false when binary is missing');
    assert.equal(res.raw.code, 127, 'exit code should be 127 on spawn error');
    assert.match(res.raw.stderr || '', /Failed to spawn committy binary/, 'stderr should mention spawn failure');
  } finally {
    if (old === undefined) delete process.env.COMMITTY_BIN; else process.env.COMMITTY_BIN = old;
  }
}

async function testGenerateGuidelines() {
  const tmp = await mkdtemp(path.join(os.tmpdir(), 'committy-guidelines-'));
  // Create files
  await writeFile(path.join(tmp, 'README.md'), '# Test Repo\n');
  await mkdir(path.join(tmp, '.github'), { recursive: true });
  await writeFile(path.join(tmp, 'CONTRIBUTING.md'), 'Contribute here');
  await writeFile(path.join(tmp, '.github', 'changelog-config.json'), '{"preset":"conventional"}');

  const res = await generateGuidelines(tmp);
  assert.ok(res.readme && res.readme.includes('# Test Repo'), 'should read README.md');
  assert.ok(res.contributing && res.contributing.includes('Contribute'), 'should read CONTRIBUTING.md');
  assert.ok(res.changelogConfig && res.changelogConfig.includes('conventional'), 'should read changelog config');
}

(async function main() {
  try {
    await testFormatMessage();
    await testMissingBinary();
    await testGenerateGuidelines();
    await testFakeCliComputeAndApplyTag();
    await testFakeCliGroupCommitPlanApply();
    await testBadJsonGroupCommit();
    await testNonZeroExitWithJson();
    console.log('All tests passed');
  } catch (err) {
    console.error('Test failed:', err);
    process.exit(1);
  }
})();
