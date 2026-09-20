//! Behaviour matrix for the local agent guardrail.
//!
//! `.claude/hooks/block-dangerous-git.sh` is a PreToolUse hook: it reads the
//! tool input as JSON on stdin and exits 2 to refuse the command. Nothing
//! executed it before, so its deny rules were unverified — which matters more
//! than for ordinary code, because a guardrail that silently stops refusing
//! looks exactly like one that works.
//!
//! Exit codes: 0 = allowed, 2 = blocked.

#![cfg(unix)]

use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

fn hook_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".claude/hooks/block-dangerous-git.sh")
}

/// Feed a command to the hook exactly as the harness would, and report whether
/// it was refused.
fn is_blocked(command: &str) -> bool {
    let payload = serde_json::json!({ "tool_input": { "command": command } }).to_string();

    let mut child = Command::new("bash")
        .arg(hook_path())
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("failed to run the guardrail hook");

    child
        .stdin
        .take()
        .expect("hook stdin")
        .write_all(payload.as_bytes())
        .expect("failed to write hook input");

    let status = child.wait().expect("hook did not exit");
    match status.code() {
        Some(0) => false,
        Some(2) => true,
        other => panic!("hook returned unexpected exit code {other:?} for: {command}"),
    }
}

// ---------------------------------------------------------------------------
// Deny side — the reason the hook exists
// ---------------------------------------------------------------------------

#[test]
fn refuses_unguarded_force_push() {
    for command in [
        "git push --force origin main",
        "git push -f origin main",
        "git push origin main --force",
    ] {
        assert!(is_blocked(command), "must refuse: {command}");
    }
}

#[test]
fn refuses_hard_reset() {
    assert!(is_blocked("git reset --hard HEAD~1"));
    assert!(is_blocked("git reset --hard origin/main"));
}

#[test]
fn refuses_forced_clean() {
    for command in ["git clean -f", "git clean -fd", "git clean -xfd"] {
        assert!(is_blocked(command), "must refuse: {command}");
    }
}

#[test]
fn refuses_forced_branch_delete() {
    assert!(is_blocked("git branch -D some-branch"));
}

#[test]
fn refuses_bulk_working_tree_discard() {
    for command in ["git checkout .", "git restore .", "git checkout -- ."] {
        assert!(is_blocked(command), "must refuse: {command}");
    }
}

/// The old hook matched raw substrings, so these evaded it entirely.
#[test]
fn refuses_destructive_commands_that_evaded_substring_matching() {
    for command in [
        "git  reset --hard HEAD~1",          // two spaces
        "env FOO=1 git reset --hard HEAD~1", // leading assignment
        "git -C . push --force origin main", // -C before the subcommand
        "cd /tmp && git clean -fd",          // not at the start of the command
    ] {
        assert!(is_blocked(command), "must refuse: {command}");
    }
}

// ---------------------------------------------------------------------------
// Allow side — over-blocking is what deadlocked this repo's own workflow
// ---------------------------------------------------------------------------

#[test]
fn allows_ordinary_pushes() {
    for command in [
        "git push",
        "git push -u origin my-branch",
        "git push origin --delete stale-branch",
        "git push --force-with-lease origin my-branch",
    ] {
        assert!(!is_blocked(command), "must allow: {command}");
    }
}

#[test]
fn allows_non_destructive_git() {
    for command in [
        "git status --short",
        "git commit -m 'feat: something'",
        "git checkout -b new-branch",
        "git restore --staged src/main.rs",
        "git branch -d merged-branch",
        "git reset HEAD~1",
    ] {
        assert!(!is_blocked(command), "must allow: {command}");
    }
}

#[test]
fn allows_commands_that_are_not_git() {
    for command in ["ls -la", "cargo test", "rm -rf target"] {
        assert!(!is_blocked(command), "must allow: {command}");
    }
}

/// The failure that motivated the rewrite: the old hook matched the *text* of
/// a command, so writing documentation that merely named one was refused —
/// including, circularly, the heredoc that would have replaced the hook.
#[test]
fn allows_a_heredoc_that_writes_prose_naming_a_dangerous_command() {
    let command = "cat > notes.md <<'EOF'\n\
                   Run git push --force to overwrite.\n\
                   git reset --hard undoes work.\n\
                   EOF";
    assert!(
        !is_blocked(command),
        "a heredoc written to a file is data, not an invocation"
    );
}

/// A heredoc fed to an interpreter *is* executed, so its body must still be
/// scanned. Dropping every heredoc body would make this a trivial bypass.
#[test]
fn still_refuses_a_destructive_command_inside_an_interpreter_heredoc() {
    for command in [
        "bash <<'EOF'\ngit reset --hard HEAD~1\nEOF",
        "sh <<EOF\ngit clean -fd\nEOF",
    ] {
        assert!(
            is_blocked(command),
            "a heredoc feeding a shell is executed, not documentation: {command}"
        );
    }
}

/// Deliberate over-block, recorded so it is a decision rather than a surprise.
///
/// Prose inside a quoted string is still refused. Stripping quoted segments
/// would fix it, but would also let `bash -c "git reset --hard"` through, and
/// for a guardrail a false refusal costs far less than a false pass.
#[test]
fn still_refuses_quoted_prose_naming_a_dangerous_command() {
    assert!(
        is_blocked("echo 'see git clean -fd' > /tmp/note.txt"),
        "quoted text is not distinguishable from `bash -c \"...\"`, so it stays blocked"
    );
}
