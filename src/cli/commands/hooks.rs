use crate::cli::output::{MachineContext, API_VERSION};
use crate::cli::Command;
use crate::error::CliError;
use crate::git;
use crate::linter::{check_message_format_for_repo, CommitIssue, CommitLinter};
use serde::Serialize;
use std::fs;
use std::io::{self, Read};
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use structopt::StructOpt;

const COMMIT_MSG_HOOK: &str = r#"#!/bin/sh
exec committy --non-interactive -q hooks run commit-msg --message-file "$1" --repo-path .
"#;

const PRE_PUSH_HOOK: &str = r#"#!/bin/sh
exec committy --non-interactive -q hooks run pre-push --repo-path .
"#;

const GITHUB_WORKFLOW: &str = r#"name: Commit conventions

on:
  pull_request:

jobs:
  lint-commits:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0
      - name: Install Committy
        run: cargo install committy --locked
      - name: Lint pull request commits
        run: committy --non-interactive lint --from-ref "${{ github.event.pull_request.base.sha }}" --to-ref "${{ github.sha }}"
"#;

#[derive(Debug, StructOpt)]
pub struct HooksCommand {
    #[structopt(subcommand)]
    action: HooksAction,
}

#[derive(Debug, StructOpt)]
enum HooksAction {
    #[structopt(name = "install")]
    Install(HooksInstallCommand),
    #[structopt(name = "run")]
    Run(HooksRunCommand),
}

#[derive(Debug, StructOpt)]
struct HooksInstallCommand {
    #[structopt(long, help = "Also install a GitHub Actions commit lint workflow")]
    with_ci: bool,
    #[structopt(long, help = "Replace existing hook and workflow files")]
    force: bool,
    #[structopt(long, help = "Preview files without writing them")]
    dry_run: bool,
    #[structopt(long, default_value = "json", possible_values = &["text", "json"])]
    output: String,
    #[structopt(long, default_value = ".", parse(from_os_str))]
    repo_path: PathBuf,
}

#[derive(Debug, StructOpt)]
struct HooksRunCommand {
    #[structopt(subcommand)]
    hook: HookRunner,
}

#[derive(Debug, StructOpt)]
enum HookRunner {
    #[structopt(name = "commit-msg")]
    CommitMsg(CommitMsgHookCommand),
    #[structopt(name = "pre-push")]
    PrePush(PrePushHookCommand),
}

#[derive(Debug, StructOpt)]
struct CommitMsgHookCommand {
    #[structopt(long = "message-file", parse(from_os_str))]
    message_file: PathBuf,
    #[structopt(long, default_value = "text", possible_values = &["text", "json"])]
    output: String,
    #[structopt(long, default_value = ".", parse(from_os_str))]
    repo_path: PathBuf,
}

#[derive(Debug, StructOpt)]
struct PrePushHookCommand {
    #[structopt(long, default_value = "text", possible_values = &["text", "json"])]
    output: String,
    #[structopt(long, default_value = ".", parse(from_os_str))]
    repo_path: PathBuf,
}

#[derive(Debug, Serialize)]
struct HooksInstallOutput {
    api_version: u8,
    command: &'static str,
    ok: bool,
    dry_run: bool,
    installed: bool,
    files: Vec<String>,
    errors: Option<Vec<String>>,
}

#[derive(Debug, Serialize)]
struct HookRunOutput<'a, T: Serialize> {
    api_version: u8,
    command: &'static str,
    hook: &'static str,
    ok: bool,
    dry_run: bool,
    count: usize,
    issues: &'a [T],
    errors: Option<Vec<String>>,
}

impl Command for HooksCommand {
    fn execute(&self, _non_interactive: bool) -> Result<(), CliError> {
        match &self.action {
            HooksAction::Install(command) => command.execute(),
            HooksAction::Run(command) => command.execute(),
        }
    }

    fn machine_context(&self) -> Option<MachineContext> {
        let (json, dry_run) = match &self.action {
            HooksAction::Install(command) => (command.output == "json", command.dry_run),
            HooksAction::Run(HooksRunCommand {
                hook: HookRunner::CommitMsg(command),
            }) => (command.output == "json", false),
            HooksAction::Run(HooksRunCommand {
                hook: HookRunner::PrePush(command),
            }) => (command.output == "json", false),
        };
        json.then_some(MachineContext {
            command: "hooks",
            dry_run,
        })
    }
}

impl HooksInstallCommand {
    fn execute(&self) -> Result<(), CliError> {
        let repo = git::discover_repository_from(&self.repo_path)?;
        let workdir = repo
            .workdir()
            .ok_or_else(|| CliError::GitError(git2::Error::from_str("No working directory")))?;
        let hooks_dir = repo.commondir().join("hooks");
        let mut files = vec![
            (hooks_dir.join("commit-msg"), COMMIT_MSG_HOOK),
            (hooks_dir.join("pre-push"), PRE_PUSH_HOOK),
        ];
        if self.with_ci {
            files.push((
                workdir.join(".github/workflows/committy.yml"),
                GITHUB_WORKFLOW,
            ));
        }

        if !self.force {
            if let Some((path, _)) = files.iter().find(|(path, _)| path.exists()) {
                return Err(CliError::InputError(format!(
                    "Refusing to replace existing file {}. Use --force to replace it.",
                    path.display()
                )));
            }
        }

        let display_files = files
            .iter()
            .map(|(path, _)| display_path(workdir, path))
            .collect::<Vec<_>>();
        if !self.dry_run {
            for (path, contents) in &files {
                if let Some(parent) = path.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::write(path, contents)?;
                if path.starts_with(&hooks_dir) {
                    set_hook_executable(path)?;
                }
            }
        }

        let payload = HooksInstallOutput {
            api_version: API_VERSION,
            command: "hooks",
            ok: true,
            dry_run: self.dry_run,
            installed: !self.dry_run,
            files: display_files,
            errors: None,
        };
        if self.output == "json" {
            println!("{}", serde_json::to_string(&payload).unwrap());
        } else if self.dry_run {
            println!("Would install {} enforcement files", payload.files.len());
        } else {
            println!("Installed {} enforcement files", payload.files.len());
        }
        Ok(())
    }
}

#[cfg(unix)]
fn set_hook_executable(path: &Path) -> Result<(), CliError> {
    let mut permissions = fs::metadata(path)?.permissions();
    permissions.set_mode(permissions.mode() | 0o755);
    fs::set_permissions(path, permissions)?;
    Ok(())
}

#[cfg(not(unix))]
fn set_hook_executable(_path: &Path) -> Result<(), CliError> {
    Ok(())
}

impl HooksRunCommand {
    fn execute(&self) -> Result<(), CliError> {
        match &self.hook {
            HookRunner::CommitMsg(command) => command.execute(),
            HookRunner::PrePush(command) => command.execute(),
        }
    }
}

impl CommitMsgHookCommand {
    fn execute(&self) -> Result<(), CliError> {
        let message = fs::read_to_string(&self.message_file)?;
        let issues = check_message_format_for_repo(&self.repo_path, &message)
            .map_err(|error| CliError::Generic(error.to_string()))?;
        print_hook_result("commit-msg", &self.output, &issues)?;
        if issues.is_empty() {
            Ok(())
        } else {
            Err(CliError::LintIssues(issues.len()))
        }
    }
}

impl PrePushHookCommand {
    fn execute(&self) -> Result<(), CliError> {
        let mut input = String::new();
        io::stdin().read_to_string(&mut input)?;
        let repo_path = self
            .repo_path
            .to_str()
            .ok_or_else(|| CliError::InputError("Invalid repo path".to_string()))?;
        let linter =
            CommitLinter::new(repo_path).map_err(|error| CliError::Generic(error.to_string()))?;
        let mut issues = Vec::<CommitIssue>::new();
        for line in input.lines() {
            let fields = line.split_whitespace().collect::<Vec<_>>();
            if fields.len() != 4 {
                return Err(CliError::InputError(
                    "Expected pre-push input: <local-ref> <local-sha> <remote-ref> <remote-sha>"
                        .to_string(),
                ));
            }
            let local_sha = fields[1];
            let remote_sha = fields[3];
            if is_zero_oid(local_sha) {
                continue;
            }
            let from_ref = (!is_zero_oid(remote_sha)).then_some(remote_sha);
            issues.extend(
                linter
                    .check_rev_range(from_ref, Some(local_sha))
                    .map_err(|error| CliError::Generic(error.to_string()))?,
            );
        }
        print_hook_result("pre-push", &self.output, &issues)?;
        if issues.is_empty() {
            Ok(())
        } else {
            Err(CliError::LintIssues(issues.len()))
        }
    }
}

fn print_hook_result<T: Serialize>(
    hook: &'static str,
    output: &str,
    issues: &[T],
) -> Result<(), CliError> {
    if output == "json" {
        println!(
            "{}",
            serde_json::to_string(&HookRunOutput {
                api_version: API_VERSION,
                command: "hooks",
                hook,
                ok: issues.is_empty(),
                dry_run: false,
                count: issues.len(),
                issues,
                errors: None,
            })
            .map_err(|error| CliError::Generic(error.to_string()))?
        );
    } else if issues.is_empty() {
        println!("{hook} policy passed");
    } else {
        eprintln!("{hook} policy found {} issue(s)", issues.len());
    }
    Ok(())
}

fn display_path(workdir: &Path, path: &Path) -> String {
    path.strip_prefix(workdir)
        .unwrap_or(path)
        .display()
        .to_string()
}

fn is_zero_oid(value: &str) -> bool {
    !value.is_empty() && value.bytes().all(|byte| byte == b'0')
}
